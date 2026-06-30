use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::application::agent_runtime::AgentRegistration;
use crate::application::model_router::ModelRouter;
use crate::application::persona::{Persona, PersonaCatalog, PersonaError};
use crate::domain::models::message::Message;
use crate::domain::ports::llm_provider::LlmProvider;
use crate::domain::ports::role::{Role, RoleError};

#[derive(Debug, Error)]
pub enum PersonaOperationsError {
    #[error("Invalid persona catalog: {0}")]
    InvalidPersonaCatalog(#[from] PersonaError),
    #[error("Persona not found in catalog: {0}")]
    PersonaNotFound(String),
}

#[derive(Default)]
struct PersonaRoleState {
    should_respond: bool,
    observed_message: Option<Message>,
    generated_content: Option<String>,
}

pub struct PersonaRuntimeRole {
    persona: Persona,
    llm_provider: Arc<dyn LlmProvider>,
    state: Arc<Mutex<PersonaRoleState>>,
}

impl std::fmt::Debug for PersonaRuntimeRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PersonaRuntimeRole")
            .field("persona", &self.persona)
            .finish()
    }
}

impl PersonaRuntimeRole {
    fn new(persona: Persona, llm_provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            persona,
            llm_provider,
            state: Arc::new(Mutex::new(PersonaRoleState::default())),
        }
    }

    fn build_prompt(&self, message: &Message) -> String {
        format!(
            "Persona: {}\nPurpose: {}\nResponsibilities: {}\nReceived message: {}",
            self.persona.name,
            self.persona.purpose,
            self.persona.responsibilities.join("; "),
            message.content()
        )
    }
}

#[async_trait]
impl Role for PersonaRuntimeRole {
    fn name(&self) -> &str {
        &self.persona.name
    }

    fn profile(&self) -> &str {
        &self.persona.purpose
    }

    async fn observe(&self, messages: &[Message]) -> Result<(), RoleError> {
        let mut state = self.state.lock().await;
        let Some(latest) = messages.last() else {
            return Ok(());
        };

        if latest.sender() == "user" {
            state.should_respond = true;
            state.observed_message = Some(latest.clone());
        }

        Ok(())
    }

    async fn think(&self) -> Result<(), RoleError> {
        let message_to_analyze = {
            let state = self.state.lock().await;
            if !state.should_respond {
                return Ok(());
            }
            state.observed_message.clone()
        };

        let Some(message) = message_to_analyze else {
            return Ok(());
        };

        let prompt = self.build_prompt(&message);
        let generated = self.llm_provider.text_only(&prompt).await?;

        let mut state = self.state.lock().await;
        if state.should_respond {
            state.generated_content = Some(generated);
        }

        Ok(())
    }

    async fn act(&self) -> Result<Option<Message>, RoleError> {
        let mut state = self.state.lock().await;
        if !state.should_respond {
            return Ok(None);
        }

        let Some(observed) = state.observed_message.clone() else {
            state.should_respond = false;
            return Ok(None);
        };

        let generated = state
            .generated_content
            .clone()
            .unwrap_or_else(|| "No analysis available".to_string());

        state.should_respond = false;
        state.observed_message = None;
        state.generated_content = None;

        Ok(Some(Message::new(
            self.persona.name.clone(),
            generated,
            Some(observed.id()),
        )))
    }
}

/// Interview-time state for the single-voice cognitive Maestro interview.
#[derive(Default)]
struct InterviewRoleState {
    should_respond: bool,
    observed_message: Option<Message>,
    generated_content: Option<String>,
    /// Recent user/Maestro turns (excluding the message being answered) so the
    /// interview retains memory across turns instead of treating each answer in
    /// isolation.
    transcript: Vec<String>,
}

/// Single-voice cognitive interview role (Option B).
///
/// Drives the onboarding interview on the same `observe → think → act` loop as
/// every other role, but with a capability-asserting prompt so Maestro presents
/// as a file-authoring orchestrator rather than a text-only assistant. It is the
/// sole producer of "Maestro" messages during the LLM-driven interview, removing
/// the historical dual-voice contradiction.
pub struct MaestroInterviewRole {
    persona: Persona,
    llm_provider: Arc<dyn LlmProvider>,
    state: Arc<Mutex<InterviewRoleState>>,
}

impl std::fmt::Debug for MaestroInterviewRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MaestroInterviewRole")
            .field("persona", &self.persona.name)
            .finish()
    }
}

impl MaestroInterviewRole {
    /// Build the interview role from an explicit Maestro persona and provider.
    pub fn new(persona: Persona, llm_provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            persona,
            llm_provider,
            state: Arc::new(Mutex::new(InterviewRoleState::default())),
        }
    }

    /// Build the interview role bound to the immutable Maestro persona.
    pub fn for_maestro(llm_provider: Arc<dyn LlmProvider>) -> Result<Self, PersonaOperationsError> {
        let catalog = PersonaCatalog::default_personas();
        catalog.validate()?;
        let persona = catalog
            .personas
            .into_iter()
            .find(|persona| persona.name == "Maestro")
            .ok_or_else(|| PersonaOperationsError::PersonaNotFound("Maestro".to_string()))?;
        Ok(Self::new(persona, llm_provider))
    }

    /// Capability-aware interview prompt that asserts file-authoring authority.
    fn build_prompt(&self, message: &Message, transcript: &[String]) -> String {
        let history = if transcript.is_empty() {
            String::new()
        } else {
            format!("\n\nConversation so far:\n{}", transcript.join("\n"))
        };
        format!(
            "{}\n\nPersona: {}\nPurpose: {}\nResponsibilities: {}{}\n\nUser said: {}\n\nRespond as Maestro with your next onboarding message. Do not impose creating a persona or file. If you have gathered meaningful context, summarize what you know so far and ask whether it is enough to proceed to handoff. Only if the user has already confirmed it is enough, emit a short confirmation followed by a fenced json proposal of the files to create. If the user said it is not enough, offer a short numbered list of next-step options to choose from. Otherwise, ask one focused question.",
            crate::application::interview_bot::maestro_capability_preamble(),
            self.persona.name,
            self.persona.purpose,
            self.persona.responsibilities.join("; "),
            history,
            message.content()
        )
    }
}

#[async_trait]
impl Role for MaestroInterviewRole {
    fn name(&self) -> &str {
        &self.persona.name
    }

    fn profile(&self) -> &str {
        &self.persona.purpose
    }

    async fn observe(&self, messages: &[Message]) -> Result<(), RoleError> {
        let mut state = self.state.lock().await;
        let Some(latest) = messages.last() else {
            return Ok(());
        };

        if latest.sender() == "user" {
            state.should_respond = true;
            state.observed_message = Some(latest.clone());
            // Snapshot recent user/Maestro turns (excluding the message being
            // answered) so the interview keeps context across turns.
            let prior = messages.len().saturating_sub(1);
            let mut recent = messages[..prior]
                .iter()
                .filter(|msg| {
                    msg.sender().eq_ignore_ascii_case("user")
                        || msg.sender().eq_ignore_ascii_case("maestro")
                })
                .rev()
                .take(12)
                .map(|msg| format!("{}: {}", msg.sender(), msg.content()))
                .collect::<Vec<_>>();
            recent.reverse();
            state.transcript = recent;
        }

        Ok(())
    }

    async fn think(&self) -> Result<(), RoleError> {
        let (message_to_analyze, transcript) = {
            let state = self.state.lock().await;
            if !state.should_respond {
                return Ok(());
            }
            (state.observed_message.clone(), state.transcript.clone())
        };

        let Some(message) = message_to_analyze else {
            return Ok(());
        };

        let prompt = self.build_prompt(&message, &transcript);
        let generated = self.llm_provider.text_only(&prompt).await?;

        let mut state = self.state.lock().await;
        if state.should_respond {
            state.generated_content = Some(generated);
        }

        Ok(())
    }

    async fn act(&self) -> Result<Option<Message>, RoleError> {
        let mut state = self.state.lock().await;
        if !state.should_respond {
            return Ok(None);
        }

        let Some(observed) = state.observed_message.clone() else {
            state.should_respond = false;
            return Ok(None);
        };

        let generated = state
            .generated_content
            .clone()
            .unwrap_or_else(|| "No analysis available".to_string());

        state.should_respond = false;
        state.observed_message = None;
        state.generated_content = None;

        Ok(Some(Message::new(
            self.persona.name.clone(),
            generated,
            Some(observed.id()),
        )))
    }
}

pub fn registrations_from_default_personas(
    router: &ModelRouter,
) -> Result<Vec<AgentRegistration>, PersonaOperationsError> {
    let catalog = PersonaCatalog::default_personas();
    catalog.validate()?;

    let registrations = catalog
        .personas
        .into_iter()
        .map(|persona| {
            let provider = router.route_for(&persona.name);
            AgentRegistration {
                name: persona.name.clone(),
                role: Arc::new(PersonaRuntimeRole::new(persona, provider)),
            }
        })
        .collect::<Vec<_>>();

    Ok(registrations)
}

/// Build runtime registrations from the governed persona catalog, resolving
/// Architect Mode edits into the live agent set. Falls back to in-code defaults when
/// governance is empty or invalid (handled inside `PersonaCatalog::from_governance`).
/// Each persona is routed to its assigned model via the [`ModelRouter`].
pub fn registrations_from_governance(
    router: &ModelRouter,
    governance: &crate::application::markdown_governance::MarkdownGovernance,
) -> Vec<AgentRegistration> {
    PersonaCatalog::from_governance(governance)
        .personas
        .into_iter()
        .map(|persona| {
            let provider = router.route_for(&persona.name);
            AgentRegistration {
                name: persona.name.clone(),
                role: Arc::new(PersonaRuntimeRole::new(persona, provider)),
            }
        })
        .collect::<Vec<_>>()
}

pub fn registrations_from_selected_personas(
    router: &ModelRouter,
    selected_names: &[&str],
) -> Result<Vec<AgentRegistration>, PersonaOperationsError> {
    let catalog = PersonaCatalog::default_personas();
    catalog.validate()?;

    let selected = selected_names
        .iter()
        .map(|name| name.trim().to_string())
        .collect::<HashSet<_>>();

    for name in &selected {
        let exists = catalog.personas.iter().any(|persona| persona.name == *name);
        if !exists {
            return Err(PersonaOperationsError::PersonaNotFound(name.clone()));
        }
    }

    let registrations = catalog
        .personas
        .into_iter()
        .filter(|persona| selected.contains(&persona.name))
        .map(|persona| {
            let provider = router.route_for(&persona.name);
            AgentRegistration {
                name: persona.name.clone(),
                role: Arc::new(PersonaRuntimeRole::new(persona, provider)),
            }
        })
        .collect::<Vec<_>>();

    Ok(registrations)
}

/// Build the single-agent registration that drives the LLM-led onboarding
/// interview.
///
/// Unlike [`registrations_from_selected_personas`], this binds the
/// capability-aware [`MaestroInterviewRole`] so the live Maestro asserts its
/// file-authoring authority, conducts the interview one question at a time, and
/// emits governed-change proposals as fenced `json` blocks — instead of the
/// generic, text-only [`PersonaRuntimeRole`] that merely echoes persona prose.
pub fn registrations_for_interview(
    router: &ModelRouter,
) -> Result<Vec<AgentRegistration>, PersonaOperationsError> {
    let provider = router.route_for("Maestro");
    let role = MaestroInterviewRole::for_maestro(provider)?;
    Ok(vec![AgentRegistration {
        name: "Maestro".to_string(),
        role: Arc::new(role),
    }])
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use tokio::time::{sleep, Duration};

    use crate::application::agent_runtime::{AgentHealth, AgentRuntime};
    use crate::application::environment::Environment;

    use super::*;

    async fn wait_until_default_personas_ready(runtime: &AgentRuntime) -> bool {
        let expected = [
            "Maestro",
            "Project Manager",
            "Quality Assurance",
            "User Experience",
            "Software Engineer",
        ];

        for _ in 0..80 {
            let snapshot = runtime.health_snapshot().await;
            let all_ready = expected.iter().all(|name| {
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

    struct DummyLlmProvider;

    #[async_trait]
    impl LlmProvider for DummyLlmProvider {
        async fn chat(
            &self,
            request: crate::domain::ports::llm_provider::LlmRequest,
        ) -> Result<crate::domain::ports::llm_provider::LlmResponse, RoleError> {
            let prompt = request
                .messages
                .first()
                .map(|m| m.content.as_str())
                .unwrap_or("");
            Ok(crate::domain::ports::llm_provider::LlmResponse {
                text: Some(format!("analysis: {}", prompt.lines().next().unwrap_or(""))),
                tool_calls: vec![],
                finish_reason: "stop".to_string(),
                usage: None,
            })
        }

        fn capabilities(&self) -> crate::domain::ports::llm_provider::ProviderCapabilities {
            crate::domain::ports::llm_provider::ProviderCapabilities::default()
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn default_personas_collaborate_on_user_message() {
        let environment = Arc::new(Environment::new(64));
        let runtime = AgentRuntime::new(Arc::clone(&environment));
        let provider: Arc<dyn LlmProvider> = Arc::new(DummyLlmProvider);
        let router = crate::application::model_router::ModelRouter::uniform(
            provider,
            crate::application::model_router::ModelAssignmentLabel {
                provider: "test".to_string(),
                model: "dummy".to_string(),
            },
        );

        let registrations = registrations_from_default_personas(&router);
        assert!(registrations.is_ok());

        let started = runtime
            .start_agents(registrations.unwrap_or_default())
            .await;
        assert!(started.is_ok());

        let ready = wait_until_default_personas_ready(&runtime).await;
        assert!(ready);

        let published = environment
            .publish(Message::new(
                "user".to_string(),
                "Plan the increment delivery".to_string(),
                None,
            ))
            .await;
        assert!(published.is_ok());

        for _ in 0..80 {
            let history = environment.get_history().await;
            if history.len() >= 5 {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }

        let history = environment.get_history().await;
        assert!(history.len() >= 5);

        let senders = history
            .iter()
            .map(|message| message.sender().to_string())
            .collect::<HashSet<_>>();

        assert!(senders.contains("user"));
        assert!(senders.contains("Maestro"));
        assert!(senders.contains("Project Manager"));
        assert!(senders.contains("Quality Assurance"));
        assert!(senders.contains("User Experience"));
        assert!(senders.contains("Software Engineer"));

        let health = runtime.health_snapshot().await;
        assert!(!matches!(health.get("Maestro"), Some(AgentHealth::Failed)));
        assert!(!matches!(
            health.get("Project Manager"),
            Some(AgentHealth::Failed)
        ));
        assert!(!matches!(
            health.get("Quality Assurance"),
            Some(AgentHealth::Failed)
        ));
        assert!(!matches!(
            health.get("User Experience"),
            Some(AgentHealth::Failed)
        ));
        assert!(!matches!(
            health.get("Software Engineer"),
            Some(AgentHealth::Failed)
        ));

        let stopped = runtime.stop_all().await;
        assert!(stopped.is_ok());
    }

    #[test]
    fn selected_persona_registration_returns_only_requested_persona() {
        let provider: Arc<dyn LlmProvider> = Arc::new(DummyLlmProvider);
        let router = crate::application::model_router::ModelRouter::uniform(
            provider,
            crate::application::model_router::ModelAssignmentLabel {
                provider: "test".to_string(),
                model: "dummy".to_string(),
            },
        );
        let registrations = registrations_from_selected_personas(&router, &["Maestro"]);
        assert!(registrations.is_ok());

        let registrations = registrations.unwrap_or_default();
        assert_eq!(registrations.len(), 1);
        assert_eq!(registrations[0].name, "Maestro");
    }

    struct CapturingProvider {
        last_prompt: Arc<Mutex<Option<String>>>,
        reply: String,
    }

    #[async_trait]
    impl LlmProvider for CapturingProvider {
        async fn chat(
            &self,
            request: crate::domain::ports::llm_provider::LlmRequest,
        ) -> Result<crate::domain::ports::llm_provider::LlmResponse, RoleError> {
            let prompt = request
                .messages
                .first()
                .map(|m| m.content.clone())
                .unwrap_or_default();
            *self.last_prompt.lock().await = Some(prompt);
            Ok(crate::domain::ports::llm_provider::LlmResponse {
                text: Some(self.reply.clone()),
                tool_calls: vec![],
                finish_reason: "stop".to_string(),
                usage: None,
            })
        }

        fn capabilities(&self) -> crate::domain::ports::llm_provider::ProviderCapabilities {
            crate::domain::ports::llm_provider::ProviderCapabilities::default()
        }
    }

    #[tokio::test]
    async fn maestro_interview_role_is_single_capability_aware_voice() {
        let last_prompt = Arc::new(Mutex::new(None));
        let provider: Arc<dyn LlmProvider> = Arc::new(CapturingProvider {
            last_prompt: Arc::clone(&last_prompt),
            reply: "What is your project's primary goal?".to_string(),
        });

        let role = MaestroInterviewRole::for_maestro(provider).expect("maestro persona exists");
        assert_eq!(role.name(), "Maestro");

        let user_message = Message::new("user".to_string(), "I want to start".to_string(), None);

        role.observe(std::slice::from_ref(&user_message))
            .await
            .expect("observe");
        role.think().await.expect("think");
        let produced = role.act().await.expect("act");

        // Exactly one Maestro-authored message is produced (single voice).
        let message = produced.expect("interview role must respond to the user");
        assert_eq!(message.sender(), "Maestro");
        assert_eq!(message.content(), "What is your project's primary goal?");

        // The prompt asserted file-authoring capability (no text-only disclaimer).
        let captured = last_prompt.lock().await.clone().expect("prompt captured");
        assert!(captured.contains("Create, Read, Update, Edit, and Delete"));

        // A second act with no new user input stays silent (no double-posting).
        let silent = role.act().await.expect("act idempotent");
        assert!(silent.is_none());
    }

    #[tokio::test]
    async fn registrations_for_interview_wire_capability_aware_maestro() {
        let last_prompt = Arc::new(Mutex::new(None));
        let proposal = "Sounds good — here is the plan:\n```json\n{\"changes\":[{\"op\":\"create\",\"kind\":\"scope\",\"name\":\"Checkout\",\"file\":\"001-checkout.md\",\"content\":\"# Checkout\"}]}\n```";
        let provider: Arc<dyn LlmProvider> = Arc::new(CapturingProvider {
            last_prompt: Arc::clone(&last_prompt),
            reply: proposal.to_string(),
        });
        let router = crate::application::model_router::ModelRouter::uniform(
            provider,
            crate::application::model_router::ModelAssignmentLabel {
                provider: "test".to_string(),
                model: "dummy".to_string(),
            },
        );

        let registrations =
            registrations_for_interview(&router).expect("interview registration builds");
        assert_eq!(registrations.len(), 1);
        assert_eq!(registrations[0].name, "Maestro");

        // Drive the wired role exactly as the runtime would.
        let role = Arc::clone(&registrations[0].role);
        let user = Message::new(
            "user".to_string(),
            "I want a checkout service".to_string(),
            None,
        );
        role.observe(std::slice::from_ref(&user))
            .await
            .expect("observe");
        role.think().await.expect("think");
        let produced = role.act().await.expect("act").expect("maestro responds");

        // The wired role uses the capability preamble, not the generic persona prompt.
        let captured = last_prompt.lock().await.clone().expect("prompt captured");
        assert!(captured.contains("Create, Read, Update, Edit, and Delete"));

        // Its reply carries a governed-change proposal the interview loop can stage.
        let parsed =
            crate::application::interview_bot::parse_directive_proposals(produced.content());
        assert_eq!(parsed.unwrap_or_default().len(), 1);
    }

    #[tokio::test]
    async fn interview_role_carries_conversation_memory() {
        let last_prompt = Arc::new(Mutex::new(None));
        let provider: Arc<dyn LlmProvider> = Arc::new(CapturingProvider {
            last_prompt: Arc::clone(&last_prompt),
            reply: "And what is your timeline?".to_string(),
        });
        let role = MaestroInterviewRole::for_maestro(provider).expect("maestro persona exists");

        let history = vec![
            Message::new(
                "Maestro".to_string(),
                "What are you building?".to_string(),
                None,
            ),
            Message::new("user".to_string(), "A checkout API".to_string(), None),
            Message::new("user".to_string(), "Using Rust".to_string(), None),
        ];

        role.observe(&history).await.expect("observe");
        role.think().await.expect("think");
        let _ = role.act().await.expect("act");

        let captured = last_prompt.lock().await.clone().expect("prompt captured");
        assert!(captured.contains("Conversation so far:"));
        // Prior turns are remembered, and the latest user turn is the focus.
        assert!(captured.contains("A checkout API"));
        assert!(captured.contains("Using Rust"));
    }
}
