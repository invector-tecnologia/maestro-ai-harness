use std::collections::HashMap;
use std::sync::Arc;

use crate::domain::ports::llm_provider::LlmProvider;

/// Human-readable label of which provider + model an agent is routed to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelAssignmentLabel {
    pub provider: String,
    pub model: String,
}

impl ModelAssignmentLabel {
    /// Compact `provider:model` descriptor for logging and reports.
    pub fn descriptor(&self) -> String {
        format!("{}:{}", self.provider, self.model)
    }
}

/// Maestro's delegation-routing authority: resolves the concrete LLM provider
/// each persona-agent should use during orchestration. Agents without an
/// explicit assignment fall back to the system default route, preserving
/// per-agent isolation — a missing assignment never blocks the harness.
#[derive(Clone)]
pub struct ModelRouter {
    routes: HashMap<String, Arc<dyn LlmProvider>>,
    default_provider: Arc<dyn LlmProvider>,
    assignments: HashMap<String, ModelAssignmentLabel>,
    default_label: ModelAssignmentLabel,
}

impl ModelRouter {
    pub fn new(
        routes: HashMap<String, Arc<dyn LlmProvider>>,
        default_provider: Arc<dyn LlmProvider>,
        assignments: HashMap<String, ModelAssignmentLabel>,
        default_label: ModelAssignmentLabel,
    ) -> Self {
        Self {
            routes,
            default_provider,
            assignments,
            default_label,
        }
    }

    /// Build a router that sends every agent to a single provider with no
    /// per-agent assignments. Useful for headless runs and tests.
    pub fn uniform(provider: Arc<dyn LlmProvider>, default_label: ModelAssignmentLabel) -> Self {
        Self {
            routes: HashMap::new(),
            default_provider: provider,
            assignments: HashMap::new(),
            default_label,
        }
    }

    /// Resolve the provider for a persona, falling back to the default route.
    pub fn route_for(&self, persona_name: &str) -> Arc<dyn LlmProvider> {
        self.routes
            .get(persona_name)
            .cloned()
            .unwrap_or_else(|| Arc::clone(&self.default_provider))
    }

    /// The provider backing the system default route.
    pub fn default_provider(&self) -> Arc<dyn LlmProvider> {
        Arc::clone(&self.default_provider)
    }

    /// The model label assigned to a persona, or the default label when the
    /// persona has no explicit assignment.
    pub fn label_for(&self, persona_name: &str) -> &ModelAssignmentLabel {
        self.assignments
            .get(persona_name)
            .unwrap_or(&self.default_label)
    }

    /// The system default route label.
    pub fn default_label(&self) -> &ModelAssignmentLabel {
        &self.default_label
    }

    /// Explicit per-agent assignments (excludes the default fallback), for
    /// observability and reporting.
    pub fn assignments(&self) -> &HashMap<String, ModelAssignmentLabel> {
        &self.assignments
    }
}

impl std::fmt::Debug for ModelRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelRouter")
            .field("assignments", &self.assignments)
            .field("default_label", &self.default_label)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    use crate::domain::ports::llm_provider::{LlmRequest, LlmResponse, ProviderCapabilities};
    use crate::domain::ports::role::RoleError;

    struct LabelProvider {
        label: String,
    }

    #[async_trait]
    impl LlmProvider for LabelProvider {
        async fn chat(&self, _request: LlmRequest) -> Result<LlmResponse, RoleError> {
            Ok(LlmResponse {
                text: Some(self.label.clone()),
                tool_calls: vec![],
                finish_reason: "stop".to_string(),
                usage: None,
            })
        }

        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }
    }

    fn provider(label: &str) -> Arc<dyn LlmProvider> {
        Arc::new(LabelProvider {
            label: label.to_string(),
        })
    }

    fn label(provider: &str, model: &str) -> ModelAssignmentLabel {
        ModelAssignmentLabel {
            provider: provider.to_string(),
            model: model.to_string(),
        }
    }

    #[tokio::test]
    async fn routes_assigned_persona_to_its_model_and_others_to_default() {
        let default = provider("default");
        let maestro = provider("maestro-model");

        let mut routes: HashMap<String, Arc<dyn LlmProvider>> = HashMap::new();
        routes.insert("Maestro".to_string(), Arc::clone(&maestro));

        let mut assignments = HashMap::new();
        assignments.insert("Maestro".to_string(), label("openai", "gpt-4-turbo"));

        let router = ModelRouter::new(routes, default, assignments, label("ollama", "mistral"));

        let maestro_reply = router.route_for("Maestro").text_only("hi").await;
        assert!(matches!(maestro_reply, Ok(ref value) if value == "maestro-model"));

        let worker_reply = router.route_for("Software Engineer").text_only("hi").await;
        assert!(matches!(worker_reply, Ok(ref value) if value == "default"));

        assert_eq!(
            router.label_for("Maestro").descriptor(),
            "openai:gpt-4-turbo"
        );
        assert_eq!(
            router.label_for("Software Engineer").descriptor(),
            "ollama:mistral"
        );
    }

    #[tokio::test]
    async fn uniform_router_sends_everyone_to_one_provider() {
        let router = ModelRouter::uniform(provider("only"), label("ollama", "mistral"));

        let a = router.route_for("Maestro").text_only("hi").await;
        let b = router.route_for("Quality Assurance").text_only("hi").await;

        assert!(matches!(a, Ok(ref value) if value == "only"));
        assert!(matches!(b, Ok(ref value) if value == "only"));
        assert!(router.assignments().is_empty());
    }
}
