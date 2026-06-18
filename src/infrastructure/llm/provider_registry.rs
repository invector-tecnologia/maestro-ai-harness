use std::collections::HashMap;
use std::sync::Arc;

use thiserror::Error;

use crate::application::config::{AppConfig, ProviderConfig};
use crate::domain::ports::llm_provider::LlmProvider;
use crate::infrastructure::llm::gemini_adapter::GeminiAdapter;
use crate::infrastructure::llm::ollama_adapter::OllamaAdapter;

type ProviderFactory = fn(&ProviderConfig) -> Result<Arc<dyn LlmProvider>, ProviderRegistryError>;

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
    #[error("Provider nao encontrado na configuracao: {0}")]
    UnknownProvider(String),
    #[error("Fabrica nao registrada para provider: {0}")]
    FactoryNotRegistered(String),
    #[error("Fabrica duplicada para provider: {0}")]
    FactoryAlreadyRegistered(String),
    #[error("Modelo {model} nao existe no provider {provider}")]
    ModelNotConfigured { provider: String, model: String },
    #[error("Configuracao inconsistente: {0}")]
    InconsistentConfig(String),
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    pub fn register(
        &mut self,
        provider_name: &str,
        factory: ProviderFactory,
    ) -> Result<(), ProviderRegistryError> {
        if self.factories.contains_key(provider_name) {
            return Err(ProviderRegistryError::FactoryAlreadyRegistered(
                provider_name.to_string(),
            ));
        }

        self.factories.insert(provider_name.to_string(), factory);
        Ok(())
    }

    pub fn register_builtin_providers(&mut self) -> Result<(), ProviderRegistryError> {
        self.register("ollama", OllamaAdapter::from_provider_config)?;
        self.register("gemini", GeminiAdapter::from_provider_config)
    }

    pub fn build(
        &self,
        provider_name: &str,
        config: &AppConfig,
    ) -> Result<Arc<dyn LlmProvider>, ProviderRegistryError> {
        let provider_cfg = config
            .providers
            .iter()
            .find(|provider| provider.name == provider_name)
            .ok_or_else(|| ProviderRegistryError::UnknownProvider(provider_name.to_string()))?;

        self.build_from_provider_config(provider_cfg)
    }

    pub fn resolve_default(
        &self,
        config: &AppConfig,
    ) -> Result<ResolvedProvider, ProviderRegistryError> {
        let provider_cfg = config
            .providers
            .iter()
            .find(|provider| provider.name == config.runtime.default_provider)
            .ok_or_else(|| {
                ProviderRegistryError::InconsistentConfig(format!(
                    "runtime.default_provider inexistente: {}",
                    config.runtime.default_provider
                ))
            })?;

        if !provider_cfg
            .models
            .iter()
            .any(|model| model == &config.runtime.default_model)
        {
            return Err(ProviderRegistryError::ModelNotConfigured {
                provider: provider_cfg.name.clone(),
                model: config.runtime.default_model.clone(),
            });
        }

        let provider = self.build_from_provider_config(provider_cfg)?;

        Ok(ResolvedProvider {
            provider_name: provider_cfg.name.clone(),
            model: config.runtime.default_model.clone(),
            provider,
        })
    }

    fn build_from_provider_config(
        &self,
        provider_cfg: &ProviderConfig,
    ) -> Result<Arc<dyn LlmProvider>, ProviderRegistryError> {
        let factory = self.factories.get(&provider_cfg.name).ok_or_else(|| {
            ProviderRegistryError::FactoryNotRegistered(provider_cfg.name.clone())
        })?;

        factory(provider_cfg)
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

    use crate::application::config::{AuthMode, RuntimePolicy};
    use crate::domain::ports::role::RoleError;

    struct DummyProvider;

    #[async_trait]
    impl LlmProvider for DummyProvider {
        async fn generate_completion(&self, prompt: &str) -> Result<String, RoleError> {
            Ok(format!("dummy:{prompt}"))
        }
    }

    fn dummy_factory(
        _provider: &ProviderConfig,
    ) -> Result<Arc<dyn LlmProvider>, ProviderRegistryError> {
        Ok(Arc::new(DummyProvider))
    }

    fn sample_config() -> AppConfig {
        AppConfig {
            providers: vec![ProviderConfig {
                name: "ollama".to_string(),
                endpoint: "http://127.0.0.1:11434/v1".to_string(),
                auth_mode: AuthMode::None,
                auth_env_var: None,
                auth_token: None,
                timeout_ms: 5000,
                models: vec!["deepseek-coder-v2".to_string()],
                max_context_chars: 128000,
            }],
            runtime: RuntimePolicy {
                retry_max_attempts: 3,
                max_concurrency: 4,
                rate_limit_per_minute: 120,
                default_provider: "ollama".to_string(),
                default_model: "deepseek-coder-v2".to_string(),
            },
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
            assert_eq!(resolved_provider.model, "deepseek-coder-v2");

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
        config.runtime.default_model = "inexistente".to_string();

        let result = registry.resolve_default(&config);

        assert!(matches!(
            result,
            Err(ProviderRegistryError::ModelNotConfigured { provider, model })
                if provider == "ollama" && model == "inexistente"
        ));
    }

    #[test]
    fn returns_error_when_factory_is_not_registered() {
        let registry = ProviderRegistry::new();
        let config = sample_config();

        let result = registry.resolve_default(&config);

        assert!(matches!(
            result,
            Err(ProviderRegistryError::FactoryNotRegistered(name)) if name == "ollama"
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
            Err(ProviderRegistryError::FactoryAlreadyRegistered(name)) if name == "ollama"
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
}
