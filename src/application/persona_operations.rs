use std::sync::Arc;

use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::application::agent_runtime::AgentRegistration;
use crate::application::persona::{Persona, PersonaCatalog, PersonaError};
use crate::domain::models::message::Message;
use crate::domain::ports::llm_provider::LlmProvider;
use crate::domain::ports::role::{Role, RoleError};

#[derive(Debug, Error)]
pub enum PersonaOperationsError {
    #[error("Catalogo de personas invalido: {0}")]
    InvalidPersonaCatalog(#[from] PersonaError),
}

#[derive(Default)]
struct PersonaRoleState {
    should_respond: bool,
    observed_message: Option<Message>,
    generated_content: Option<String>,
    handoff_index: usize,
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
            "Persona: {}\nProposito: {}\nResponsabilidades: {}\nMensagem recebida: {}",
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
        let generated = self.llm_provider.generate_completion(&prompt).await?;

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
            .unwrap_or_else(|| "Sem analise disponivel".to_string());

        let interaction = &self.persona.interaction_matrix
            [state.handoff_index % self.persona.interaction_matrix.len()];

        state.handoff_index = state.handoff_index.saturating_add(1);
        state.should_respond = false;
        state.observed_message = None;
        state.generated_content = None;

        let content = format!(
            "{} | Handoff para {}: {}",
            generated, interaction.target_persona, interaction.expected_handoff
        );

        Ok(Some(Message::new(
            self.persona.name.clone(),
            content,
            Some(observed.id()),
        )))
    }
}

pub fn registrations_from_default_personas(
    llm_provider: Arc<dyn LlmProvider>,
) -> Result<Vec<AgentRegistration>, PersonaOperationsError> {
    let catalog = PersonaCatalog::default_personas();
    catalog.validate()?;

    let registrations = catalog
        .personas
        .into_iter()
        .map(|persona| AgentRegistration {
            name: persona.name.clone(),
            role: Arc::new(PersonaRuntimeRole::new(persona, Arc::clone(&llm_provider))),
        })
        .collect::<Vec<_>>();

    Ok(registrations)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use tokio::time::{sleep, Duration};

    use crate::application::agent_runtime::{AgentHealth, AgentRuntime};
    use crate::application::environment::Environment;

    use super::*;

    async fn wait_until_default_personas_ready(runtime: &AgentRuntime) -> bool {
        let expected = ["Product", "Engineering", "UX", "DevOps"];

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
        async fn generate_completion(&self, prompt: &str) -> Result<String, RoleError> {
            Ok(format!("analise: {}", prompt.lines().next().unwrap_or("")))
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn default_personas_collaborate_on_user_message() {
        let environment = Arc::new(Environment::new(64));
        let runtime = AgentRuntime::new(Arc::clone(&environment));
        let provider: Arc<dyn LlmProvider> = Arc::new(DummyLlmProvider);

        let registrations = registrations_from_default_personas(provider);
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
                "Planejar entrega do incremento".to_string(),
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
        assert!(senders.contains("Product"));
        assert!(senders.contains("Engineering"));
        assert!(senders.contains("UX"));
        assert!(senders.contains("DevOps"));

        let health = runtime.health_snapshot().await;
        assert!(!matches!(health.get("Product"), Some(AgentHealth::Failed)));
        assert!(!matches!(
            health.get("Engineering"),
            Some(AgentHealth::Failed)
        ));
        assert!(!matches!(health.get("UX"), Some(AgentHealth::Failed)));
        assert!(!matches!(health.get("DevOps"), Some(AgentHealth::Failed)));

        let stopped = runtime.stop_all().await;
        assert!(stopped.is_ok());
    }
}
