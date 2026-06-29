use std::collections::HashMap;
use std::sync::Arc;

use thiserror::Error;

use crate::application::config::{AppConfig, ProviderConfig};
use crate::application::model_router::{ModelAssignmentLabel, ModelRouter};
use crate::domain::ports::llm_provider::LlmProvider;
use crate::infrastructure::llm::anthropic_adapter::AnthropicAdapter;
use crate::infrastructure::llm::gemini_adapter::GeminiAdapter;
use crate::infrastructure::llm::ollama_adapter::OllamaAdapter;
use crate::infrastructure::llm::openai_adapter::OpenAiAdapter;

type ProviderFactory =
    fn(&ProviderConfig, &str) -> Result<Arc<dyn LlmProvider>, ProviderRegistryError>;

pub struct ProviderRegistry {
    factories: HashMap<String, ProviderFactory>,
}

pub struct ResolvedProvider {
    pub provider_name: String,
    pub model: String,
    pub provider: Arc<dyn LlmProvider>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ProviderRegistryError {
    #[error("Provider not found in config: {0}")]
    UnknownProvider(String),
    #[error("Factory not registered for kind: {0}")]
    FactoryNotRegistered(String),
    #[error("Duplicate factory for kind: {0}")]
    FactoryAlreadyRegistered(String),
    #[error("Model {model} does not exist in provider {provider}")]
    ModelNotConfigured { provider: String, model: String },
    #[error("Inconsistent config: {0}")]
    InconsistentConfig(String),
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Register a factory for a provider kind (e.g., "openai", "anthropic", "ollama", "gemini")
    pub fn register(
        &mut self,
        kind: &str,
        factory: ProviderFactory,
    ) -> Result<(), ProviderRegistryError> {
        if self.factories.contains_key(kind) {
            return Err(ProviderRegistryError::FactoryAlreadyRegistered(
                kind.to_string(),
            ));
        }

        self.factories.insert(kind.to_string(), factory);
        Ok(())
    }

    pub fn register_builtin_providers(&mut self) -> Result<(), ProviderRegistryError> {
        self.register("openai", OpenAiAdapter::from_provider_config)?;
        self.register("anthropic", AnthropicAdapter::from_provider_config)?;
        self.register("ollama", OllamaAdapter::from_provider_config)?;
        self.register("gemini", GeminiAdapter::from_provider_config)
    }

    /// Resolve a provider by name from config (routes to factory by kind field).
    /// Binds the provider's first configured model; prefer `resolve` for an
    /// explicit model.
    pub fn build(
        &self,
        provider_name: &str,
        config: &AppConfig,
    ) -> Result<Arc<dyn LlmProvider>, ProviderRegistryError> {
        let provider_cfg = config
            .providers
            .get(provider_name)
            .ok_or_else(|| ProviderRegistryError::UnknownProvider(provider_name.to_string()))?;

        let model = provider_cfg
            .models
            .first()
            .map(|m| m.name.clone())
            .ok_or_else(|| {
                ProviderRegistryError::InconsistentConfig(format!(
                    "provider '{}' has no models",
                    provider_name
                ))
            })?;

        self.build_from_provider_config(provider_cfg, &model)
    }

    /// Resolve a specific provider + model from config, binding the adapter to
    /// exactly the requested model.
    pub fn resolve(
        &self,
        provider_name: &str,
        model: &str,
        config: &AppConfig,
    ) -> Result<ResolvedProvider, ProviderRegistryError> {
        let provider_cfg = config
            .providers
            .get(provider_name)
            .ok_or_else(|| ProviderRegistryError::UnknownProvider(provider_name.to_string()))?;

        if !provider_cfg.models.iter().any(|m| m.name == model) {
            return Err(ProviderRegistryError::ModelNotConfigured {
                provider: provider_name.to_string(),
                model: model.to_string(),
            });
        }

        let provider = self.build_from_provider_config(provider_cfg, model)?;

        Ok(ResolvedProvider {
            provider_name: provider_name.to_string(),
            model: model.to_string(),
            provider,
        })
    }

    /// Resolve default provider from config
    pub fn resolve_default(
        &self,
        config: &AppConfig,
    ) -> Result<ResolvedProvider, ProviderRegistryError> {
        let default_name = config.system.default_provider.clone();
        let default_model = config.system.default_model.clone();

        if !config.providers.contains_key(&default_name) {
            return Err(ProviderRegistryError::InconsistentConfig(format!(
                "system.default_provider not found: {}",
                default_name
            )));
        }

        self.resolve(&default_name, &default_model, config)
    }

    /// Build the per-agent model router: the orchestrator's routing authority.
    /// Resolves the system default route plus every explicit `agents` assignment,
    /// sharing one adapter instance across agents bound to the same provider+model.
    pub fn build_model_router(
        &self,
        config: &AppConfig,
    ) -> Result<ModelRouter, ProviderRegistryError> {
        let default = self.resolve_default(config)?;
        let default_label = ModelAssignmentLabel {
            provider: default.provider_name.clone(),
            model: default.model.clone(),
        };

        let mut cache: HashMap<(String, String), Arc<dyn LlmProvider>> = HashMap::new();
        cache.insert(
            (default.provider_name.clone(), default.model.clone()),
            Arc::clone(&default.provider),
        );

        let mut routes: HashMap<String, Arc<dyn LlmProvider>> = HashMap::new();
        let mut assignments: HashMap<String, ModelAssignmentLabel> = HashMap::new();

        for (persona, assignment) in &config.agents {
            let key = (assignment.provider.clone(), assignment.model.clone());
            let provider = match cache.get(&key) {
                Some(existing) => Arc::clone(existing),
                None => {
                    let resolved = self.resolve(&assignment.provider, &assignment.model, config)?;
                    cache.insert(key, Arc::clone(&resolved.provider));
                    resolved.provider
                }
            };
            routes.insert(persona.clone(), provider);
            assignments.insert(
                persona.clone(),
                ModelAssignmentLabel {
                    provider: assignment.provider.clone(),
                    model: assignment.model.clone(),
                },
            );
        }

        Ok(ModelRouter::new(
            routes,
            default.provider,
            assignments,
            default_label,
        ))
    }

    /// Build all providers from config
    pub fn build_all(
        &self,
        config: &AppConfig,
    ) -> Result<HashMap<String, Arc<dyn LlmProvider>>, ProviderRegistryError> {
        let mut providers = HashMap::new();

        for (name, provider_cfg) in &config.providers {
            let model = provider_cfg
                .models
                .first()
                .map(|m| m.name.clone())
                .ok_or_else(|| {
                    ProviderRegistryError::InconsistentConfig(format!(
                        "provider '{}' has no models",
                        name
                    ))
                })?;
            let provider = self.build_from_provider_config(provider_cfg, &model)?;
            providers.insert(name.clone(), provider);
        }

        Ok(providers)
    }

    fn build_from_provider_config(
        &self,
        provider_cfg: &ProviderConfig,
        model: &str,
    ) -> Result<Arc<dyn LlmProvider>, ProviderRegistryError> {
        let factory = self.factories.get(&provider_cfg.kind).ok_or_else(|| {
            ProviderRegistryError::FactoryNotRegistered(provider_cfg.kind.clone())
        })?;

        factory(provider_cfg, model)
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;

    use crate::application::config::{
        AgentModelAssignment, AuthMode, ModelSpec, ProviderConfig, SystemPolicy,
    };
    use crate::domain::ports::llm_provider::{LlmRequest, LlmResponse, ProviderCapabilities};
    use crate::domain::ports::role::RoleError;

    struct DummyProvider;

    #[async_trait]
    impl LlmProvider for DummyProvider {
        async fn chat(&self, request: LlmRequest) -> Result<LlmResponse, RoleError> {
            let prompt = request
                .messages
                .first()
                .map(|m| m.content.clone())
                .unwrap_or_default();
            Ok(LlmResponse {
                text: Some(format!("dummy:{}", prompt)),
                tool_calls: vec![],
                finish_reason: "stop".to_string(),
                usage: None,
            })
        }

        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::default()
        }
    }

    fn dummy_factory(
        _provider: &ProviderConfig,
        _model: &str,
    ) -> Result<Arc<dyn LlmProvider>, ProviderRegistryError> {
        Ok(Arc::new(DummyProvider))
    }

    fn sample_config() -> AppConfig {
        let mut providers = HashMap::new();
        providers.insert(
            "ollama".to_string(),
            ProviderConfig {
                kind: "ollama".to_string(),
                endpoint: "http://127.0.0.1:11434/v1".to_string(),
                auth_mode: AuthMode::None,
                auth_env_var: None,
                timeout_ms: 5000,
                models: vec![ModelSpec {
                    name: "mistral".to_string(),
                    context_window: 32000,
                }],
                capabilities: ProviderCapabilities::default(),
            },
        );

        AppConfig {
            system: SystemPolicy {
                default_provider: "ollama".to_string(),
                default_model: "mistral".to_string(),
                max_concurrency: 4,
                rate_limit_per_minute: 120,
                retry_max_attempts: 3,
            },
            providers,
            agents: HashMap::new(),
        }
    }

    #[tokio::test]
    async fn resolves_default_provider_from_config() {
        let mut registry = ProviderRegistry::new();
        let registered = registry.register("ollama", dummy_factory);
        assert!(registered.is_ok());

        let config = sample_config();
        let resolved = registry.resolve_default(&config);

        assert!(resolved.is_ok());
        if let Ok(resolved_provider) = resolved {
            assert_eq!(resolved_provider.provider_name, "ollama");
            assert_eq!(resolved_provider.model, "mistral");

            let completion = resolved_provider.provider.generate_completion("ping").await;
            assert!(matches!(completion, Ok(ref value) if value == "dummy:ping"));
        }
    }

    #[test]
    fn returns_error_when_provider_is_missing_in_config() {
        let registry = ProviderRegistry::new();
        let config = sample_config();

        let result = registry.build("openai", &config);

        assert!(matches!(
            result,
            Err(ProviderRegistryError::UnknownProvider(name)) if name == "openai"
        ));
    }

    #[test]
    fn returns_error_when_model_is_missing_for_default_provider() {
        let mut registry = ProviderRegistry::new();
        let registered = registry.register("ollama", dummy_factory);
        assert!(registered.is_ok());

        let mut config = sample_config();
        config.system.default_model = "missing-model".to_string();

        let result = registry.resolve_default(&config);

        assert!(matches!(
            result,
            Err(ProviderRegistryError::ModelNotConfigured { provider, model })
                if provider == "ollama" && model == "missing-model"
        ));
    }

    #[test]
    fn returns_error_when_factory_is_not_registered() {
        let registry = ProviderRegistry::new();
        let config = sample_config();

        let result = registry.resolve_default(&config);

        assert!(matches!(
            result,
            Err(ProviderRegistryError::FactoryNotRegistered(kind)) if kind == "ollama"
        ));
    }

    #[test]
    fn rejects_duplicate_factory_registration() {
        let mut registry = ProviderRegistry::new();
        let first = registry.register("ollama", dummy_factory);
        assert!(first.is_ok());

        let second = registry.register("ollama", dummy_factory);

        assert!(matches!(
            second,
            Err(ProviderRegistryError::FactoryAlreadyRegistered(kind)) if kind == "ollama"
        ));
    }

    #[test]
    fn builtin_registration_includes_ollama_factory() {
        let mut registry = ProviderRegistry::new();
        let registered = registry.register_builtin_providers();
        assert!(registered.is_ok());

        let config = sample_config();
        let resolved = registry.resolve_default(&config);

        assert!(resolved.is_ok());
    }

    fn multi_provider_config() -> AppConfig {
        let mut cfg = sample_config();
        cfg.providers.insert(
            "openai".to_string(),
            ProviderConfig {
                kind: "openai".to_string(),
                endpoint: "https://api.openai.com/v1".to_string(),
                auth_mode: AuthMode::None,
                auth_env_var: None,
                timeout_ms: 5000,
                models: vec![
                    ModelSpec {
                        name: "gpt-4".to_string(),
                        context_window: 8192,
                    },
                    ModelSpec {
                        name: "gpt-4-turbo".to_string(),
                        context_window: 128000,
                    },
                ],
                capabilities: ProviderCapabilities::default(),
            },
        );
        cfg
    }

    #[test]
    fn resolve_binds_requested_model_not_first() {
        let mut registry = ProviderRegistry::new();
        let registered = registry.register("openai", dummy_factory);
        assert!(registered.is_ok());

        let config = multi_provider_config();
        let resolved = registry.resolve("openai", "gpt-4-turbo", &config);

        assert!(resolved.is_ok());
        if let Ok(value) = resolved {
            assert_eq!(value.provider_name, "openai");
            assert_eq!(value.model, "gpt-4-turbo");
        }
    }

    #[test]
    fn resolve_rejects_model_absent_from_provider() {
        let mut registry = ProviderRegistry::new();
        let registered = registry.register("openai", dummy_factory);
        assert!(registered.is_ok());

        let config = multi_provider_config();
        let result = registry.resolve("openai", "not-a-model", &config);

        assert!(matches!(
            result,
            Err(ProviderRegistryError::ModelNotConfigured { provider, model })
                if provider == "openai" && model == "not-a-model"
        ));
    }

    #[test]
    fn build_model_router_assigns_agent_models_and_defaults_others() {
        let mut registry = ProviderRegistry::new();
        let ollama = registry.register("ollama", dummy_factory);
        assert!(ollama.is_ok());
        let openai = registry.register("openai", dummy_factory);
        assert!(openai.is_ok());

        let mut config = multi_provider_config();
        config.agents.insert(
            "Software Engineer".to_string(),
            AgentModelAssignment {
                provider: "openai".to_string(),
                model: "gpt-4-turbo".to_string(),
            },
        );

        let router = registry.build_model_router(&config);
        assert!(router.is_ok());
        if let Ok(router) = router {
            let assigned = router.label_for("Software Engineer");
            assert_eq!(assigned.provider, "openai");
            assert_eq!(assigned.model, "gpt-4-turbo");

            // Unlisted personas fall back to the system default route.
            let fallback = router.label_for("Researcher");
            assert_eq!(fallback.provider, "ollama");
            assert_eq!(fallback.model, "mistral");
        }
    }

    #[test]
    fn build_model_router_shares_one_adapter_for_identical_assignments() {
        let mut registry = ProviderRegistry::new();
        let ollama = registry.register("ollama", dummy_factory);
        assert!(ollama.is_ok());
        let openai = registry.register("openai", dummy_factory);
        assert!(openai.is_ok());

        let mut config = multi_provider_config();
        config.agents.insert(
            "Engineer A".to_string(),
            AgentModelAssignment {
                provider: "openai".to_string(),
                model: "gpt-4-turbo".to_string(),
            },
        );
        config.agents.insert(
            "Engineer B".to_string(),
            AgentModelAssignment {
                provider: "openai".to_string(),
                model: "gpt-4-turbo".to_string(),
            },
        );

        let router = registry.build_model_router(&config);
        assert!(router.is_ok());
        if let Ok(router) = router {
            let a = router.route_for("Engineer A");
            let b = router.route_for("Engineer B");
            assert!(Arc::ptr_eq(&a, &b));
        }
    }
}
