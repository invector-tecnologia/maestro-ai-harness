use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::domain::ports::llm_provider::ProviderCapabilities;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub system: SystemPolicy,
    pub providers: HashMap<String, ProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPolicy {
    pub default_provider: String,
    pub default_model: String,
    pub max_concurrency: u32,
    pub rate_limit_per_minute: u32,
    pub retry_max_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub kind: String,
    pub endpoint: String,
    pub auth_mode: AuthMode,
    pub auth_env_var: Option<String>,
    pub timeout_ms: u64,
    pub models: Vec<ModelSpec>,
    pub capabilities: ProviderCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSpec {
    pub name: String,
    pub context_window: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    None,
    Bearer,
    Browser,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Config file not found: {0}")]
    NotFound(String),
    #[error("Invalid YAML syntax: {0}")]
    YamlError(#[from] serde_yaml::Error),
    #[error("Invalid config: {0}")]
    Validation(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub const DEFAULT_CONFIG_TEMPLATE: &str = "system:\n  default_provider: \"ollama\"\n  default_model: \"mistral\"\n  max_concurrency: 4\n  rate_limit_per_minute: 120\n  retry_max_attempts: 3\n\nproviders:\n  ollama:\n    kind: \"ollama\"\n    endpoint: \"http://localhost:11434/v1\"\n    auth_mode: \"none\"\n    timeout_ms: 60000\n    models:\n      - name: \"mistral\"\n        context_window: 32000\n    capabilities:\n      supports_tools: false\n      supports_streaming: true\n      supports_json_mode: false\n      supports_reasoning_controls: false\n      max_context_tokens: 32000\n";

pub struct ConfigLoader;

impl ConfigLoader {
    pub fn load(path: Option<PathBuf>) -> Result<AppConfig, ConfigError> {
        let resolved_path = match path {
            Some(p) => p,
            None => {
                let local_path = std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join("maestro")
                    .join("config.yaml");

                if local_path.exists() {
                    local_path
                } else {
                    default_config_path()?
                }
            }
        };

        if !resolved_path.exists() {
            return Err(ConfigError::NotFound(
                resolved_path.to_string_lossy().to_string(),
            ));
        }

        let content = fs::read_to_string(resolved_path)?;
        let config: AppConfig = serde_yaml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }
}

fn default_config_path() -> Result<PathBuf, ConfigError> {
    if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg_config_home.trim().is_empty() {
            return Ok(PathBuf::from(xdg_config_home)
                .join("maestro")
                .join("config.yaml"));
        }
    }

    let home = std::env::var("HOME")
        .map_err(|_| ConfigError::Validation("HOME environment variable not set".to_string()))?;

    Ok(PathBuf::from(home)
        .join(".config")
        .join("maestro")
        .join("config.yaml"))
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !self.providers.contains_key(&self.system.default_provider) {
            return Err(ConfigError::Validation(format!(
                "default_provider '{}' not found in providers",
                self.system.default_provider
            )));
        }

        let default_provider = &self.providers[&self.system.default_provider];
        if !default_provider
            .models
            .iter()
            .any(|m| m.name == self.system.default_model)
        {
            return Err(ConfigError::Validation(format!(
                "default_model '{}' not found in provider '{}'",
                self.system.default_model, self.system.default_provider
            )));
        }

        for (name, provider) in &self.providers {
            if provider.kind.is_empty() {
                return Err(ConfigError::Validation(format!(
                    "provider '{}' missing kind field",
                    name
                )));
            }
            if provider.models.is_empty() {
                return Err(ConfigError::Validation(format!(
                    "provider '{}' has no models",
                    name
                )));
            }
        }

        Ok(())
    }

    pub fn get_provider(&self, name: &str) -> Option<&ProviderConfig> {
        self.providers.get(name)
    }

    pub fn default_provider_config(&self) -> &ProviderConfig {
        &self.providers[&self.system.default_provider]
    }

    pub fn provider_names(&self) -> Vec<&String> {
        self.providers.keys().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_and_validates_config_yaml() {
        let yaml = "system:\n  default_provider: \"openai\"\n  default_model: \"gpt-4\"\n  max_concurrency: 4\n  rate_limit_per_minute: 60\n  retry_max_attempts: 3\nproviders:\n  openai:\n    kind: \"openai\"\n    endpoint: \"https://api.openai.com/v1\"\n    auth_mode: \"bearer\"\n    auth_env_var: \"OPENAI_API_KEY\"\n    timeout_ms: 30000\n    models:\n      - name: \"gpt-4\"\n        context_window: 8192\n    capabilities:\n      supports_tools: true\n      supports_streaming: true\n      supports_json_mode: true\n      supports_reasoning_controls: true\n      max_context_tokens: 8192\n";

        let config: AppConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.system.default_provider, "openai");
        assert!(config.validate().is_ok());
        assert!(config.get_provider("openai").is_some());
    }

    #[test]
    fn rejects_invalid_default_provider() {
        let yaml = "system:\n  default_provider: \"nonexistent\"\n  default_model: \"gpt-4\"\n  max_concurrency: 4\n  rate_limit_per_minute: 60\n  retry_max_attempts: 3\nproviders:\n  openai:\n    kind: \"openai\"\n    endpoint: \"https://api.openai.com/v1\"\n    auth_mode: \"bearer\"\n    timeout_ms: 30000\n    models:\n      - name: \"gpt-4\"\n        context_window: 8192\n    capabilities:\n      supports_tools: true\n      supports_streaming: true\n      supports_json_mode: true\n      supports_reasoning_controls: true\n      max_context_tokens: 8192\n";

        let config: AppConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_invalid_default_model() {
        let yaml = "system:\n  default_provider: \"openai\"\n  default_model: \"nonexistent\"\n  max_concurrency: 4\n  rate_limit_per_minute: 60\n  retry_max_attempts: 3\nproviders:\n  openai:\n    kind: \"openai\"\n    endpoint: \"https://api.openai.com/v1\"\n    auth_mode: \"bearer\"\n    timeout_ms: 30000\n    models:\n      - name: \"gpt-4\"\n        context_window: 8192\n    capabilities:\n      supports_tools: true\n      supports_streaming: true\n      supports_json_mode: true\n      supports_reasoning_controls: true\n      max_context_tokens: 8192\n";

        let config: AppConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.validate().is_err());
    }
}
