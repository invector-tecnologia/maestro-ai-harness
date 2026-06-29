use std::collections::HashMap;
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::{broadcast, oneshot, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, info};

use crate::application::agent_observability::{RuntimeEvent, RuntimeEventWithTimestamp};
use crate::application::environment::{Environment, EnvironmentError};
use crate::domain::models::message::Message;
use crate::domain::ports::role::{Role, RoleError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentHealth {
    Starting,
    Idle,
    Observing,
    Thinking,
    Acting,
    Failed,
    Stopped,
}

pub struct AgentRegistration {
    pub name: String,
    pub role: Arc<dyn Role>,
}

/// Per-agent result of a sequential Maestro-orchestrated workflow run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequentialAgentOutcome {
    pub agent_name: String,
    pub succeeded: bool,
    pub heartbeats: u32,
    pub output: Option<String>,
    pub error: Option<String>,
}

/// Aggregate report for a sequential workflow run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SequentialRunReport {
    pub outcomes: Vec<SequentialAgentOutcome>,
}

impl SequentialRunReport {
    /// Agent execution order (as orchestrated).
    pub fn order(&self) -> Vec<String> {
        self.outcomes
            .iter()
            .map(|outcome| outcome.agent_name.clone())
            .collect()
    }

    /// Count of agents that completed their cycle successfully.
    pub fn completed(&self) -> usize {
        self.outcomes.iter().filter(|o| o.succeeded).count()
    }

    /// Count of agents whose cycle failed (isolated, workflow continued).
    pub fn failed(&self) -> usize {
        self.outcomes.iter().filter(|o| !o.succeeded).count()
    }
}

struct AgentTask {
    shutdown_tx: oneshot::Sender<()>,
    join_handle: JoinHandle<()>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AgentRuntimeError {
    #[error("Agent already registered: {0}")]
    AgentAlreadyRunning(String),
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    #[error("Failed to join agent task {0}")]
    JoinFailure(String),
}

pub struct AgentRuntime {
    environment: Arc<Environment>,
    tasks: Mutex<HashMap<String, AgentTask>>,
    health: Arc<RwLock<HashMap<String, AgentHealth>>>,
    event_tx: broadcast::Sender<RuntimeEventWithTimestamp>,
    event_history: Arc<RwLock<Vec<RuntimeEventWithTimestamp>>>,
}

impl AgentRuntime {
    pub fn new(environment: Arc<Environment>) -> Self {
        let (event_tx, _) = broadcast::channel(256);

        Self {
            environment,
            tasks: Mutex::new(HashMap::new()),
            health: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            event_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn start_agents(
        &self,
        registrations: Vec<AgentRegistration>,
    ) -> Result<(), AgentRuntimeError> {
        for registration in registrations {
            self.start_agent(registration).await?;
        }

        Ok(())
    }

    pub async fn start_agent(
        &self,
        registration: AgentRegistration,
    ) -> Result<(), AgentRuntimeError> {
        {
            let tasks = self.tasks.lock().await;
            if tasks.contains_key(&registration.name) {
                return Err(AgentRuntimeError::AgentAlreadyRunning(registration.name));
            }
        }

        set_health(&self.health, &registration.name, AgentHealth::Starting).await;

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let agent_name = registration.name.clone();
        let environment = Arc::clone(&self.environment);
        let health = Arc::clone(&self.health);
        let role = Arc::clone(&registration.role);
        let event_tx = self.event_tx.clone();

        let join_handle = tokio::spawn(async move {
            run_agent_loop(agent_name, role, environment, health, shutdown_rx, event_tx).await;
        });

        let mut tasks = self.tasks.lock().await;
        tasks.insert(
            registration.name,
            AgentTask {
                shutdown_tx,
                join_handle,
            },
        );

        Ok(())
    }

    pub async fn stop_agent(&self, name: &str) -> Result<(), AgentRuntimeError> {
        let task = {
            let mut tasks = self.tasks.lock().await;
            tasks.remove(name)
        };

        let Some(agent_task) = task else {
            return Err(AgentRuntimeError::AgentNotFound(name.to_string()));
        };

        let _ = agent_task.shutdown_tx.send(());
        let joined = agent_task.join_handle.await;
        if joined.is_err() {
            return Err(AgentRuntimeError::JoinFailure(name.to_string()));
        }

        set_health(&self.health, name, AgentHealth::Stopped).await;
        Ok(())
    }

    pub async fn stop_all(&self) -> Result<(), AgentRuntimeError> {
        let names = {
            let tasks = self.tasks.lock().await;
            tasks.keys().cloned().collect::<Vec<_>>()
        };

        for name in names {
            self.stop_agent(&name).await?;
        }

        Ok(())
    }

    pub async fn health_snapshot(&self) -> HashMap<String, AgentHealth> {
        self.health.read().await.clone()
    }

    /// Subscribe to runtime events for observability.
    pub fn subscribe_to_events(&self) -> broadcast::Receiver<RuntimeEventWithTimestamp> {
        self.event_tx.subscribe()
    }

    /// Emit a runtime event (internal use).
    #[allow(dead_code)]
    async fn emit_event(&self, event: RuntimeEvent) {
        let event_with_ts = RuntimeEventWithTimestamp {
            event,
            timestamp: std::time::SystemTime::now(),
        };
        let _ = self.event_tx.send(event_with_ts.clone());

        // Store in history
        let mut history = self.event_history.write().await;
        history.push(event_with_ts);
        // Keep only last 100 events
        if history.len() > 100 {
            history.remove(0);
        }
    }

    /// Get a snapshot of recent runtime events.
    pub async fn events_snapshot(&self) -> Vec<RuntimeEventWithTimestamp> {
        self.event_history.read().await.clone()
    }

    /// Orchestrate agents sequentially: each agent starts only after the
    /// previous finishes. Maestro narrates each transition and emits a
    /// heartbeat at least every `heartbeat_period` while an agent runs longer
    /// than that threshold. Per-agent failures are isolated: a failed agent is
    /// recorded and the workflow continues with the next agent.
    pub async fn orchestrate_sequential(
        &self,
        pipeline: Vec<AgentRegistration>,
        trigger: Message,
        heartbeat_period: std::time::Duration,
    ) -> SequentialRunReport {
        let event_tx = self.event_tx.clone();
        let history = Arc::clone(&self.event_history);
        let environment = Arc::clone(&self.environment);
        let health = Arc::clone(&self.health);

        let order: Vec<String> = pipeline.iter().map(|reg| reg.name.clone()).collect();
        record_event(
            &event_tx,
            &history,
            RuntimeEvent::MaestroNarration {
                agent_name: "Maestro".to_string(),
                phase: "workflow-start".to_string(),
                detail: format!(
                    "orchestrating {} agent(s): {}",
                    order.len(),
                    order.join(" → ")
                ),
            },
        )
        .await;

        let mut report = SequentialRunReport::default();
        let mut current_input = trigger;

        for registration in pipeline {
            let name = registration.name.clone();
            record_event(
                &event_tx,
                &history,
                RuntimeEvent::MaestroNarration {
                    agent_name: "Maestro".to_string(),
                    phase: "agent-start".to_string(),
                    detail: format!("starting {name}"),
                },
            )
            .await;

            let outcome = run_sequential_cycle(
                &name,
                registration.role.as_ref(),
                &current_input,
                &environment,
                &health,
                &event_tx,
                &history,
                heartbeat_period,
            )
            .await;

            if let Some(output) = &outcome.output {
                current_input =
                    Message::new(name.clone(), output.clone(), Some(current_input.id()));
            }

            record_event(
                &event_tx,
                &history,
                RuntimeEvent::MaestroNarration {
                    agent_name: "Maestro".to_string(),
                    phase: if outcome.succeeded {
                        "agent-complete".to_string()
                    } else {
                        "agent-failed".to_string()
                    },
                    detail: format!(
                        "{name} {}",
                        if outcome.succeeded {
                            "completed"
                        } else {
                            "failed (isolated)"
                        }
                    ),
                },
            )
            .await;

            report.outcomes.push(outcome);
        }

        record_event(
            &event_tx,
            &history,
            RuntimeEvent::MaestroNarration {
                agent_name: "Maestro".to_string(),
                phase: "workflow-complete".to_string(),
                detail: format!(
                    "{} completed, {} failed",
                    report.completed(),
                    report.failed()
                ),
            },
        )
        .await;

        report
    }
}

async fn run_agent_loop(
    agent_name: String,
    role: Arc<dyn Role>,
    environment: Arc<Environment>,
    health: Arc<RwLock<HashMap<String, AgentHealth>>>,
    mut shutdown_rx: oneshot::Receiver<()>,
    event_tx: broadcast::Sender<RuntimeEventWithTimestamp>,
) {
    info!(agent = %agent_name, "agente iniciado");
    set_health(&health, &agent_name, AgentHealth::Idle).await;

    let mut receiver = environment.subscribe();

    loop {
        tokio::select! {
            _ = &mut shutdown_rx => {
                info!(agent = %agent_name, "shutdown solicitado");
                set_health(&health, &agent_name, AgentHealth::Stopped).await;
                break;
            }
            received = receiver.recv() => {
                match received {
                    Ok(message) => {
                        let processed = process_message_cycle(
                            &agent_name,
                            role.as_ref(),
                            Arc::clone(&environment),
                            Arc::clone(&health),
                            message,
                            event_tx.clone(),
                        ).await;

                        if let Err(error) = processed {
                            publish_runtime_error(
                                Arc::clone(&environment),
                                event_tx.clone(),
                                &agent_name,
                                format!("agent cycle failed: {error}"),
                            )
                            .await;
                            set_health(&health, &agent_name, AgentHealth::Failed).await;
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        debug!(agent = %agent_name, skipped, "mensagens perdidas por lag");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        set_health(&health, &agent_name, AgentHealth::Stopped).await;
                        break;
                    }
                }
            }
        }
    }
}

async fn process_message_cycle(
    agent_name: &str,
    role: &dyn Role,
    environment: Arc<Environment>,
    health: Arc<RwLock<HashMap<String, AgentHealth>>>,
    message: Message,
    event_tx: broadcast::Sender<RuntimeEventWithTimestamp>,
) -> Result<(), RoleError> {
    set_health(&health, agent_name, AgentHealth::Observing).await;
    let observe_event = RuntimeEvent::AgentObserving {
        agent_name: agent_name.to_string(),
        message_id: message.id().to_string(),
    };
    let _ = event_tx.send(RuntimeEventWithTimestamp {
        event: observe_event,
        timestamp: std::time::SystemTime::now(),
    });

    if let Err(error) = role.observe(std::slice::from_ref(&message)).await {
        publish_runtime_error(
            Arc::clone(&environment),
            event_tx.clone(),
            agent_name,
            format!("observe failed: {error}"),
        )
        .await;
        return Err(error);
    }

    set_health(&health, agent_name, AgentHealth::Thinking).await;
    let think_event = RuntimeEvent::AgentThinking {
        agent_name: agent_name.to_string(),
        context: format!("Analyzing: {}", message.content()),
    };
    let _ = event_tx.send(RuntimeEventWithTimestamp {
        event: think_event,
        timestamp: std::time::SystemTime::now(),
    });

    if let Err(error) = role.think().await {
        publish_runtime_error(
            Arc::clone(&environment),
            event_tx.clone(),
            agent_name,
            format!("think failed: {error}"),
        )
        .await;
        return Err(error);
    }

    set_health(&health, agent_name, AgentHealth::Acting).await;
    let acting_event = RuntimeEvent::AgentActing {
        agent_name: agent_name.to_string(),
        decision: "Preparing response".to_string(),
    };
    let _ = event_tx.send(RuntimeEventWithTimestamp {
        event: acting_event,
        timestamp: std::time::SystemTime::now(),
    });

    let maybe_outgoing = match role.act().await {
        Ok(value) => value,
        Err(error) => {
            publish_runtime_error(
                Arc::clone(&environment),
                event_tx.clone(),
                agent_name,
                format!("act failed: {error}"),
            )
            .await;
            return Err(error);
        }
    };
    if let Some(outgoing) = maybe_outgoing {
        let acted_event = RuntimeEvent::AgentActed {
            agent_name: agent_name.to_string(),
            output: outgoing.content().to_string(),
            handoff_target: None,
        };
        let _ = event_tx.send(RuntimeEventWithTimestamp {
            event: acted_event,
            timestamp: std::time::SystemTime::now(),
        });

        let published = environment.publish(outgoing).await;
        if let Err(EnvironmentError::NoSubscribers) = published {
            debug!(agent = %agent_name, "output dropped: no subscribers");
        }
    }

    set_health(&health, agent_name, AgentHealth::Idle).await;
    Ok(())
}

async fn publish_runtime_error(
    environment: Arc<Environment>,
    event_tx: broadcast::Sender<RuntimeEventWithTimestamp>,
    agent_name: &str,
    error_message: String,
) {
    let _ = event_tx.send(RuntimeEventWithTimestamp {
        event: RuntimeEvent::ExecutionError {
            agent_name: agent_name.to_string(),
            error_message: error_message.clone(),
        },
        timestamp: std::time::SystemTime::now(),
    });

    let _ = environment
        .publish(Message::new(
            "system".to_string(),
            format!("⚠️ Agent '{agent_name}' error: {error_message}"),
            None,
        ))
        .await;
}

async fn set_health(
    health: &Arc<RwLock<HashMap<String, AgentHealth>>>,
    name: &str,
    state: AgentHealth,
) {
    let mut guard = health.write().await;
    guard.insert(name.to_string(), state);
}

/// Run a single agent's observe→think→act cycle as part of a sequential
/// workflow, emitting a heartbeat at least every `heartbeat_period` while the
/// cycle runs longer than that threshold. Failures are returned as a non-fatal
/// outcome so the orchestrator can continue with the next agent.
#[allow(clippy::too_many_arguments)]
async fn run_sequential_cycle(
    agent_name: &str,
    role: &dyn Role,
    input: &Message,
    environment: &Arc<Environment>,
    health: &Arc<RwLock<HashMap<String, AgentHealth>>>,
    event_tx: &broadcast::Sender<RuntimeEventWithTimestamp>,
    history: &Arc<RwLock<Vec<RuntimeEventWithTimestamp>>>,
    heartbeat_period: std::time::Duration,
) -> SequentialAgentOutcome {
    let started = std::time::Instant::now();
    let mut heartbeats: u32 = 0;

    set_health(health, agent_name, AgentHealth::Observing).await;

    // The observe→think→act cycle, narrating each phase between awaits.
    let cycle = async {
        record_event(
            event_tx,
            history,
            RuntimeEvent::AgentObserving {
                agent_name: agent_name.to_string(),
                message_id: input.id().to_string(),
            },
        )
        .await;
        role.observe(std::slice::from_ref(input)).await?;

        set_health(health, agent_name, AgentHealth::Thinking).await;
        record_event(
            event_tx,
            history,
            RuntimeEvent::AgentThinking {
                agent_name: agent_name.to_string(),
                context: format!("analyzing: {}", input.content()),
            },
        )
        .await;
        role.think().await?;

        set_health(health, agent_name, AgentHealth::Acting).await;
        record_event(
            event_tx,
            history,
            RuntimeEvent::AgentActing {
                agent_name: agent_name.to_string(),
                decision: "preparing response".to_string(),
            },
        )
        .await;
        role.act().await
    };
    tokio::pin!(cycle);

    let mut ticker = tokio::time::interval(heartbeat_period);
    // Consume the immediate first tick so heartbeats only fire after the period.
    ticker.tick().await;

    let cycle_result = loop {
        tokio::select! {
            result = &mut cycle => break result,
            _ = ticker.tick() => {
                if started.elapsed() >= heartbeat_period {
                    heartbeats += 1;
                    record_event(
                        event_tx,
                        history,
                        RuntimeEvent::MaestroHeartbeat {
                            agent_name: agent_name.to_string(),
                            elapsed_secs: started.elapsed().as_secs(),
                        },
                    )
                    .await;
                }
            }
        }
    };

    match cycle_result {
        Ok(maybe_outgoing) => {
            let output = maybe_outgoing.as_ref().map(|m| m.content().to_string());
            if let Some(outgoing) = maybe_outgoing {
                record_event(
                    event_tx,
                    history,
                    RuntimeEvent::AgentActed {
                        agent_name: agent_name.to_string(),
                        output: outgoing.content().to_string(),
                        handoff_target: None,
                    },
                )
                .await;
                let published = environment.publish(outgoing).await;
                if let Err(EnvironmentError::NoSubscribers) = published {
                    debug!(agent = %agent_name, "output dropped: no subscribers");
                }
            }
            set_health(health, agent_name, AgentHealth::Idle).await;
            SequentialAgentOutcome {
                agent_name: agent_name.to_string(),
                succeeded: true,
                heartbeats,
                output,
                error: None,
            }
        }
        Err(error) => {
            // Per-agent failure isolation: record and continue the workflow.
            publish_runtime_error(
                Arc::clone(environment),
                event_tx.clone(),
                agent_name,
                format!("sequential cycle failed: {error}"),
            )
            .await;
            set_health(health, agent_name, AgentHealth::Failed).await;
            SequentialAgentOutcome {
                agent_name: agent_name.to_string(),
                succeeded: false,
                heartbeats,
                output: None,
                error: Some(error.to_string()),
            }
        }
    }
}

/// Send a runtime event to subscribers and append it to the bounded history.
async fn record_event(
    event_tx: &broadcast::Sender<RuntimeEventWithTimestamp>,
    history: &Arc<RwLock<Vec<RuntimeEventWithTimestamp>>>,
    event: RuntimeEvent,
) {
    let event_with_ts = RuntimeEventWithTimestamp {
        event,
        timestamp: std::time::SystemTime::now(),
    };
    let _ = event_tx.send(event_with_ts.clone());

    let mut guard = history.write().await;
    guard.push(event_with_ts);
    if guard.len() > 100 {
        guard.remove(0);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use async_trait::async_trait;
    use tokio::time::{sleep, Duration};

    use super::*;

    async fn wait_until_agents_ready(runtime: &AgentRuntime, names: &[&str]) -> bool {
        for _ in 0..40 {
            let snapshot = runtime.health_snapshot().await;
            let all_ready = names.iter().all(|name| {
                matches!(
                    snapshot.get(*name),
                    Some(AgentHealth::Idle)
                        | Some(AgentHealth::Observing)
                        | Some(AgentHealth::Thinking)
                        | Some(AgentHealth::Acting)
                )
            });

            if all_ready {
                return true;
            }

            sleep(Duration::from_millis(10)).await;
        }

        false
    }

    struct CountingRole {
        name: String,
        counter: Arc<AtomicUsize>,
        fail_on_observe: bool,
    }

    #[async_trait]
    impl Role for CountingRole {
        fn name(&self) -> &str {
            &self.name
        }

        fn profile(&self) -> &str {
            "counting"
        }

        async fn observe(&self, _messages: &[Message]) -> Result<(), RoleError> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            if self.fail_on_observe {
                return Err(RoleError::ReasoningError);
            }
            Ok(())
        }

        async fn think(&self) -> Result<(), RoleError> {
            Ok(())
        }

        async fn act(&self) -> Result<Option<Message>, RoleError> {
            Ok(None)
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn multi_agent_runtime_processes_message_in_parallel() {
        let environment = Arc::new(Environment::new(32));
        let runtime = AgentRuntime::new(Arc::clone(&environment));

        let first_counter = Arc::new(AtomicUsize::new(0));
        let second_counter = Arc::new(AtomicUsize::new(0));

        let started = runtime
            .start_agents(vec![
                AgentRegistration {
                    name: "produto".to_string(),
                    role: Arc::new(CountingRole {
                        name: "produto".to_string(),
                        counter: Arc::clone(&first_counter),
                        fail_on_observe: false,
                    }),
                },
                AgentRegistration {
                    name: "engenharia".to_string(),
                    role: Arc::new(CountingRole {
                        name: "engenharia".to_string(),
                        counter: Arc::clone(&second_counter),
                        fail_on_observe: false,
                    }),
                },
            ])
            .await;
        assert!(started.is_ok());

        let ready = wait_until_agents_ready(&runtime, &["produto", "engenharia"]).await;
        assert!(ready);

        let message = Message::new("user".to_string(), "kickoff".to_string(), None);
        let published = environment.publish(message).await;
        assert!(published.is_ok());

        for _ in 0..20 {
            let first_seen = first_counter.load(Ordering::SeqCst);
            let second_seen = second_counter.load(Ordering::SeqCst);
            if first_seen > 0 && second_seen > 0 {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }

        assert!(first_counter.load(Ordering::SeqCst) > 0);
        assert!(second_counter.load(Ordering::SeqCst) > 0);

        let stopped = runtime.stop_all().await;
        assert!(stopped.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn failing_agent_does_not_stop_healthy_agent() {
        let environment = Arc::new(Environment::new(32));
        let runtime = AgentRuntime::new(Arc::clone(&environment));

        let healthy_counter = Arc::new(AtomicUsize::new(0));
        let failing_counter = Arc::new(AtomicUsize::new(0));

        let started = runtime
            .start_agents(vec![
                AgentRegistration {
                    name: "healthy".to_string(),
                    role: Arc::new(CountingRole {
                        name: "healthy".to_string(),
                        counter: Arc::clone(&healthy_counter),
                        fail_on_observe: false,
                    }),
                },
                AgentRegistration {
                    name: "failing".to_string(),
                    role: Arc::new(CountingRole {
                        name: "failing".to_string(),
                        counter: Arc::clone(&failing_counter),
                        fail_on_observe: true,
                    }),
                },
            ])
            .await;
        assert!(started.is_ok());

        let ready = wait_until_agents_ready(&runtime, &["healthy", "failing"]).await;
        assert!(ready);

        let first_publish = environment
            .publish(Message::new("user".to_string(), "first".to_string(), None))
            .await;
        assert!(first_publish.is_ok());

        for _ in 0..20 {
            if healthy_counter.load(Ordering::SeqCst) > 0
                && failing_counter.load(Ordering::SeqCst) > 0
            {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }

        let second_publish = environment
            .publish(Message::new("user".to_string(), "second".to_string(), None))
            .await;
        assert!(second_publish.is_ok());

        for _ in 0..20 {
            if healthy_counter.load(Ordering::SeqCst) > 1 {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }

        assert!(healthy_counter.load(Ordering::SeqCst) > 1);

        let health = runtime.health_snapshot().await;
        assert!(matches!(health.get("failing"), Some(AgentHealth::Failed)));
        assert!(!matches!(health.get("healthy"), Some(AgentHealth::Failed)));

        let stopped = runtime.stop_all().await;
        assert!(stopped.is_ok());
    }

    struct SequentialRole {
        name: String,
        order_log: Arc<std::sync::Mutex<Vec<String>>>,
        think_delay: Duration,
        fail: bool,
        output: Option<String>,
    }

    #[async_trait]
    impl Role for SequentialRole {
        fn name(&self) -> &str {
            &self.name
        }

        fn profile(&self) -> &str {
            "sequential"
        }

        async fn observe(&self, _messages: &[Message]) -> Result<(), RoleError> {
            if self.fail {
                return Err(RoleError::ReasoningError);
            }
            Ok(())
        }

        async fn think(&self) -> Result<(), RoleError> {
            if !self.think_delay.is_zero() {
                sleep(self.think_delay).await;
            }
            Ok(())
        }

        async fn act(&self) -> Result<Option<Message>, RoleError> {
            self.order_log
                .lock()
                .expect("order log poisoned")
                .push(self.name.clone());
            Ok(self
                .output
                .clone()
                .map(|content| Message::new(self.name.clone(), content, None)))
        }
    }

    fn sequential_registration(
        name: &str,
        order_log: &Arc<std::sync::Mutex<Vec<String>>>,
        think_delay: Duration,
        fail: bool,
    ) -> AgentRegistration {
        AgentRegistration {
            name: name.to_string(),
            role: Arc::new(SequentialRole {
                name: name.to_string(),
                order_log: Arc::clone(order_log),
                think_delay,
                fail,
                output: Some(format!("{name} output")),
            }),
        }
    }

    #[tokio::test]
    async fn sequential_workflow_runs_agents_in_registration_order() {
        let environment = Arc::new(Environment::new(32));
        let runtime = AgentRuntime::new(Arc::clone(&environment));
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));

        let pipeline = vec![
            sequential_registration("alpha", &log, Duration::ZERO, false),
            sequential_registration("beta", &log, Duration::ZERO, false),
            sequential_registration("gamma", &log, Duration::ZERO, false),
        ];

        let report = runtime
            .orchestrate_sequential(
                pipeline,
                Message::new("user".to_string(), "go".to_string(), None),
                Duration::from_secs(5),
            )
            .await;

        assert_eq!(report.order(), vec!["alpha", "beta", "gamma"]);
        assert_eq!(report.completed(), 3);
        assert_eq!(report.failed(), 0);
        assert_eq!(
            log.lock().expect("order log poisoned").clone(),
            vec!["alpha", "beta", "gamma"]
        );
    }

    #[tokio::test]
    async fn sequential_workflow_isolates_agent_failure() {
        let environment = Arc::new(Environment::new(32));
        let runtime = AgentRuntime::new(Arc::clone(&environment));
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));

        let pipeline = vec![
            sequential_registration("alpha", &log, Duration::ZERO, false),
            sequential_registration("beta", &log, Duration::ZERO, true),
            sequential_registration("gamma", &log, Duration::ZERO, false),
        ];

        let report = runtime
            .orchestrate_sequential(
                pipeline,
                Message::new("user".to_string(), "go".to_string(), None),
                Duration::from_secs(5),
            )
            .await;

        assert_eq!(report.order(), vec!["alpha", "beta", "gamma"]);
        assert_eq!(report.completed(), 2);
        assert_eq!(report.failed(), 1);

        let beta = report
            .outcomes
            .iter()
            .find(|outcome| outcome.agent_name == "beta")
            .expect("beta outcome present");
        assert!(!beta.succeeded);
        assert!(beta.error.is_some());

        // The failing agent never reached act(); later agents still executed.
        assert_eq!(
            log.lock().expect("order log poisoned").clone(),
            vec!["alpha", "gamma"]
        );

        let health = runtime.health_snapshot().await;
        assert!(matches!(health.get("beta"), Some(AgentHealth::Failed)));
        assert!(matches!(health.get("gamma"), Some(AgentHealth::Idle)));
    }

    #[tokio::test]
    async fn sequential_workflow_emits_heartbeat_for_slow_agent() {
        let environment = Arc::new(Environment::new(32));
        let runtime = AgentRuntime::new(Arc::clone(&environment));
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));

        let pipeline = vec![sequential_registration(
            "slowpoke",
            &log,
            Duration::from_millis(80),
            false,
        )];

        let report = runtime
            .orchestrate_sequential(
                pipeline,
                Message::new("user".to_string(), "go".to_string(), None),
                Duration::from_millis(20),
            )
            .await;

        let outcome = report.outcomes.first().expect("slowpoke outcome present");
        assert!(outcome.succeeded);
        assert!(
            outcome.heartbeats >= 1,
            "expected at least one heartbeat, got {}",
            outcome.heartbeats
        );

        let events = runtime.events_snapshot().await;
        let heartbeat_seen = events.iter().any(|entry| {
            matches!(
                &entry.event,
                RuntimeEvent::MaestroHeartbeat { agent_name, .. } if agent_name == "slowpoke"
            )
        });
        assert!(heartbeat_seen, "expected a MaestroHeartbeat event");
    }

    #[tokio::test]
    async fn sequential_workflow_narrates_each_transition() {
        let environment = Arc::new(Environment::new(32));
        let runtime = AgentRuntime::new(Arc::clone(&environment));
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));

        let pipeline = vec![
            sequential_registration("alpha", &log, Duration::ZERO, false),
            sequential_registration("beta", &log, Duration::ZERO, false),
        ];

        let _report = runtime
            .orchestrate_sequential(
                pipeline,
                Message::new("user".to_string(), "go".to_string(), None),
                Duration::from_secs(5),
            )
            .await;

        let events = runtime.events_snapshot().await;
        let phases: Vec<String> = events
            .iter()
            .filter_map(|entry| match &entry.event {
                RuntimeEvent::MaestroNarration { phase, .. } => Some(phase.clone()),
                _ => None,
            })
            .collect();

        assert!(phases.contains(&"workflow-start".to_string()));
        assert_eq!(
            phases
                .iter()
                .filter(|phase| phase.as_str() == "agent-start")
                .count(),
            2
        );
        assert_eq!(
            phases
                .iter()
                .filter(|phase| phase.as_str() == "agent-complete")
                .count(),
            2
        );
        assert!(phases.contains(&"workflow-complete".to_string()));
    }
}
