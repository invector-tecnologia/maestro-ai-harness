use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::ports::llm_provider::ProviderCapabilities;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub system: SystemPolicy,
    pub providers: HashMap<String, ProviderConfig>,
    /// Optional per-agent model assignments, keyed by persona name.
    /// Unassigned agents fall back to `system.default_provider`/`default_model`.
    #[serde(default)]
    pub agents: HashMap<String, AgentModelAssignment>,
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

/// Binds a single agent (persona) to a specific provider + model from the catalog.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentModelAssignment {
    pub provider: String,
    pub model: String,
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

pub const DEFAULT_CONFIG_TEMPLATE: &str = "system:\n  default_provider: \"ollama\"\n  default_model: \"mistral\"\n  max_concurrency: 4\n  rate_limit_per_minute: 120\n  retry_max_attempts: 3\n\nproviders:\n  ollama:\n    kind: \"ollama\"\n    endpoint: \"http://localhost:11434/v1\"\n    auth_mode: \"none\"\n    timeout_ms: 60000\n    models:\n      - name: \"mistral\"\n        context_window: 32000\n    capabilities:\n      supports_tools: false\n      supports_streaming: true\n      supports_json_mode: false\n      supports_reasoning_controls: false\n      max_context_tokens: 32000\n\n# Optional: assign specific catalog models to specific agents (personas).\n# Unassigned agents use system.default_provider/default_model.\n# agents:\n#   Maestro:\n#     provider: \"ollama\"\n#     model: \"mistral\"\n";

pub struct ConfigLoader;

/// Canonical config file name for new installs.
pub const CONFIG_FILE_NAME: &str = "config.yml";
/// Legacy config file name, still read for backward compatibility.
pub const LEGACY_CONFIG_FILE_NAME: &str = "config.yaml";

/// Returns the existing config file inside a `maestro/` directory, preferring
/// `config.yml` over a legacy `config.yaml`. `None` when neither exists. This
/// discovery helper does not emit a deprecation warning (the loader does).
pub fn existing_config_in(maestro_dir: &Path) -> Option<PathBuf> {
    let yml = maestro_dir.join(CONFIG_FILE_NAME);
    if yml.exists() {
        return Some(yml);
    }

    let yaml = maestro_dir.join(LEGACY_CONFIG_FILE_NAME);
    if yaml.exists() {
        return Some(yaml);
    }

    None
}

impl ConfigLoader {
    pub fn load(path: Option<PathBuf>) -> Result<AppConfig, ConfigError> {
        let resolved_path = match path {
            Some(p) => p,
            None => resolve_default_config_location()?,
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

/// Resolve the default config location, preferring the local `maestro/` directory
/// over the global config directory. Within each directory `config.yml` wins; a
/// legacy `config.yaml` is accepted with a deprecation warning.
fn resolve_default_config_location() -> Result<PathBuf, ConfigError> {
    let local_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("maestro");
    if let Some(path) = preferred_config_in_dir(&local_dir) {
        return Ok(path);
    }

    let global_dir = config_base_dir()?;
    if let Some(path) = preferred_config_in_dir(&global_dir) {
        return Ok(path);
    }

    // Nothing found yet: point the NotFound error at the canonical `.yml` path.
    Ok(global_dir.join("config.yml"))
}

/// Returns `config.yml` when present, otherwise a legacy `config.yaml` with a
/// deprecation warning. `None` when neither exists in `dir`.
fn preferred_config_in_dir(dir: &Path) -> Option<PathBuf> {
    let yml = dir.join("config.yml");
    if yml.exists() {
        return Some(yml);
    }

    let yaml = dir.join("config.yaml");
    if yaml.exists() {
        tracing::warn!(
            path = %yaml.display(),
            "config.yaml is deprecated; rename it to config.yml"
        );
        return Some(yaml);
    }

    None
}

fn config_base_dir() -> Result<PathBuf, ConfigError> {
    if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg_config_home.trim().is_empty() {
            return Ok(PathBuf::from(xdg_config_home).join("maestro"));
        }
    }

    let home = std::env::var("HOME")
        .map_err(|_| ConfigError::Validation("HOME environment variable not set".to_string()))?;

    Ok(PathBuf::from(home).join(".config").join("maestro"))
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

        for (agent, assignment) in &self.agents {
            let provider = self.providers.get(&assignment.provider).ok_or_else(|| {
                ConfigError::Validation(format!(
                    "agent '{}' references unknown provider '{}'",
                    agent, assignment.provider
                ))
            })?;
            if !provider.models.iter().any(|m| m.name == assignment.model) {
                return Err(ConfigError::Validation(format!(
                    "agent '{}' references model '{}' not declared in provider '{}'",
                    agent, assignment.model, assignment.provider
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

    fn config_with_agents(agents_yaml: &str) -> AppConfig {
        let yaml = format!(
            "system:\n  default_provider: \"openai\"\n  default_model: \"gpt-4\"\n  max_concurrency: 4\n  rate_limit_per_minute: 60\n  retry_max_attempts: 3\nproviders:\n  openai:\n    kind: \"openai\"\n    endpoint: \"https://api.openai.com/v1\"\n    auth_mode: \"bearer\"\n    timeout_ms: 30000\n    models:\n      - name: \"gpt-4\"\n        context_window: 8192\n    capabilities:\n      supports_tools: true\n      supports_streaming: true\n      supports_json_mode: true\n      supports_reasoning_controls: true\n      max_context_tokens: 8192\n  ollama:\n    kind: \"ollama\"\n    endpoint: \"http://localhost:11434/v1\"\n    auth_mode: \"none\"\n    timeout_ms: 60000\n    models:\n      - name: \"mistral\"\n        context_window: 32000\n    capabilities:\n      supports_tools: false\n      supports_streaming: true\n      supports_json_mode: false\n      supports_reasoning_controls: false\n      max_context_tokens: 32000\n{}",
            agents_yaml
        );
        serde_yaml::from_str(&yaml).expect("config yaml parses")
    }

    #[test]
    fn accepts_agent_assignment_to_existing_provider_model() {
        let config = config_with_agents(
            "agents:\n  \"Software Engineer\":\n    provider: \"ollama\"\n    model: \"mistral\"\n",
        );
        assert!(config.validate().is_ok());
        let assignment = config.agents.get("Software Engineer").unwrap();
        assert_eq!(assignment.provider, "ollama");
        assert_eq!(assignment.model, "mistral");
    }

    #[test]
    fn rejects_agent_referencing_unknown_provider() {
        let config = config_with_agents(
            "agents:\n  \"Researcher\":\n    provider: \"missing\"\n    model: \"mistral\"\n",
        );
        assert!(config.validate().is_err());
    }

    #[test]
    fn rejects_agent_referencing_unknown_model() {
        let config = config_with_agents(
            "agents:\n  \"Researcher\":\n    provider: \"ollama\"\n    model: \"not-a-model\"\n",
        );
        assert!(config.validate().is_err());
    }

    #[test]
    fn existing_config_in_prefers_yml_over_legacy_yaml() {
        let dir = std::env::temp_dir().join(format!(
            "maestro-config-pref-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(dir.join("config.yml"), "yml").expect("write yml");
        std::fs::write(dir.join("config.yaml"), "yaml").expect("write yaml");

        let resolved = existing_config_in(&dir);
        assert_eq!(resolved, Some(dir.join("config.yml")));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn existing_config_in_falls_back_to_legacy_yaml() {
        let dir = std::env::temp_dir().join(format!(
            "maestro-config-legacy-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(dir.join("config.yaml"), "yaml").expect("write yaml");

        let resolved = existing_config_in(&dir);
        assert_eq!(resolved, Some(dir.join("config.yaml")));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
