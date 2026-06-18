use std::collections::HashMap;
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, info};

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

struct AgentTask {
    shutdown_tx: oneshot::Sender<()>,
    join_handle: JoinHandle<()>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AgentRuntimeError {
    #[error("Agente ja registrado: {0}")]
    AgentAlreadyRunning(String),
    #[error("Agente nao encontrado: {0}")]
    AgentNotFound(String),
    #[error("Falha ao finalizar task do agente {0}")]
    JoinFailure(String),
}

pub struct AgentRuntime {
    environment: Arc<Environment>,
    tasks: Mutex<HashMap<String, AgentTask>>,
    health: Arc<RwLock<HashMap<String, AgentHealth>>>,
}

impl AgentRuntime {
    pub fn new(environment: Arc<Environment>) -> Self {
        Self {
            environment,
            tasks: Mutex::new(HashMap::new()),
            health: Arc::new(RwLock::new(HashMap::new())),
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

        let join_handle = tokio::spawn(async move {
            run_agent_loop(agent_name, role, environment, health, shutdown_rx).await;
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
}

async fn run_agent_loop(
    agent_name: String,
    role: Arc<dyn Role>,
    environment: Arc<Environment>,
    health: Arc<RwLock<HashMap<String, AgentHealth>>>,
    mut shutdown_rx: oneshot::Receiver<()>,
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
                        ).await;

                        if processed.is_err() {
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
) -> Result<(), RoleError> {
    set_health(&health, agent_name, AgentHealth::Observing).await;
    role.observe(std::slice::from_ref(&message)).await?;

    set_health(&health, agent_name, AgentHealth::Thinking).await;
    role.think().await?;

    set_health(&health, agent_name, AgentHealth::Acting).await;
    let maybe_outgoing = role.act().await?;
    if let Some(outgoing) = maybe_outgoing {
        let published = environment.publish(outgoing).await;
        if let Err(EnvironmentError::NoSubscribers) = published {
            debug!(agent = %agent_name, "saida descartada sem assinantes");
        }
    }

    set_health(&health, agent_name, AgentHealth::Idle).await;
    Ok(())
}

async fn set_health(
    health: &Arc<RwLock<HashMap<String, AgentHealth>>>,
    name: &str,
    state: AgentHealth,
) {
    let mut guard = health.write().await;
    guard.insert(name.to_string(), state);
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
}
