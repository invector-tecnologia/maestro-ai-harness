use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMode {
    None,
    Bearer,
    Browser,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfig {
    pub name: String,
    pub endpoint: String,
    pub auth_mode: AuthMode,
    pub auth_env_var: Option<String>,
    pub auth_token: Option<String>,
    pub timeout_ms: u64,
    pub models: Vec<String>,
    pub max_context_chars: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePolicy {
    pub retry_max_attempts: u8,
    pub max_concurrency: usize,
    pub rate_limit_per_minute: u32,
    pub default_provider: String,
    pub default_model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub providers: Vec<ProviderConfig>,
    pub runtime: RuntimePolicy,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Erro de IO ao carregar configuracao")]
    Io(#[from] std::io::Error),
    #[error("Falha ao parsear TOML: {0}")]
    ParseToml(#[from] toml::de::Error),
    #[error("Configuracao invalida: {0}")]
    Invalid(String),
}

#[derive(Debug, Deserialize)]
struct AppConfigFile {
    providers: Vec<ProviderConfigFile>,
    runtime: RuntimePolicyFile,
}

#[derive(Debug, Deserialize)]
struct ProviderConfigFile {
    name: String,
    endpoint: String,
    auth_mode: String,
    auth_env_var: Option<String>,
    timeout_ms: u64,
    models: Vec<String>,
    max_context_chars: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct RuntimePolicyFile {
    retry_max_attempts: u8,
    max_concurrency: usize,
    rate_limit_per_minute: u32,
    default_provider: String,
    default_model: String,
}

pub const DEFAULT_CONFIG_TEMPLATE: &str = r#"[[providers]]
name = "ollama"
endpoint = "http://127.0.0.1:11434/v1"
auth_mode = "none"
timeout_ms = 10000
models = ["deepseek-coder-v2"]
max_context_chars = 128000

[runtime]
retry_max_attempts = 3
max_concurrency = 4
rate_limit_per_minute = 120
default_provider = "ollama"
default_model = "deepseek-coder-v2"
"#;

pub struct ConfigLoader;

impl ConfigLoader {
    pub fn load(path: Option<PathBuf>) -> Result<AppConfig, ConfigError> {
        let resolved_path = match path {
            Some(p) => p,
            None => {
                let local_path = std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join("maestro")
                    .join("config.toml");

                if local_path.exists() {
                    local_path
                } else {
                    default_config_path()?
                }
            }
        };

        let raw = fs::read_to_string(resolved_path)?;
        let parsed: AppConfigFile = toml::from_str(&raw)?;
        to_validated_config(parsed)
    }
}

fn default_config_path() -> Result<PathBuf, ConfigError> {
    if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg_config_home.trim().is_empty() {
            return Ok(PathBuf::from(xdg_config_home)
                .join("maestro")
                .join("config.toml"));
        }
    }

    let home = std::env::var("HOME")
        .map_err(|_| ConfigError::Invalid("Variavel HOME nao definida".to_string()))?;

    Ok(PathBuf::from(home)
        .join(".config")
        .join("maestro")
        .join("config.toml"))
}

fn to_validated_config(raw: AppConfigFile) -> Result<AppConfig, ConfigError> {
    if raw.providers.is_empty() {
        return Err(ConfigError::Invalid(
            "A lista de providers nao pode ser vazia".to_string(),
        ));
    }

    let mut unique_names = HashSet::new();
    let mut providers = Vec::with_capacity(raw.providers.len());

    for provider in raw.providers {
        if provider.name.trim().is_empty() {
            return Err(ConfigError::Invalid("Provider com nome vazio".to_string()));
        }

        if !unique_names.insert(provider.name.clone()) {
            return Err(ConfigError::Invalid(format!(
                "Provider duplicado: {}",
                provider.name
            )));
        }

        if !(provider.endpoint.starts_with("http://") || provider.endpoint.starts_with("https://"))
        {
            return Err(ConfigError::Invalid(format!(
                "Endpoint invalido para provider {}",
                provider.name
            )));
        }

        if provider.timeout_ms == 0 {
            return Err(ConfigError::Invalid(format!(
                "timeout_ms deve ser maior que zero para provider {}",
                provider.name
            )));
        }

        if provider.models.is_empty() {
            return Err(ConfigError::Invalid(format!(
                "Provider {} deve ter ao menos um modelo",
                provider.name
            )));
        }

        let auth_mode = parse_auth_mode(&provider.auth_mode)?;
        let auth_token =
            resolve_auth_token(&provider.name, &auth_mode, provider.auth_env_var.as_deref())?;

        let max_context_chars = provider.max_context_chars.unwrap_or(128000).max(1000);

        providers.push(ProviderConfig {
            name: provider.name,
            endpoint: provider.endpoint,
            auth_mode,
            auth_env_var: provider.auth_env_var,
            auth_token,
            timeout_ms: provider.timeout_ms,
            models: provider.models,
            max_context_chars,
        });
    }

    if raw.runtime.max_concurrency == 0 {
        return Err(ConfigError::Invalid(
            "runtime.max_concurrency deve ser maior que zero".to_string(),
        ));
    }

    if raw.runtime.rate_limit_per_minute == 0 {
        return Err(ConfigError::Invalid(
            "runtime.rate_limit_per_minute deve ser maior que zero".to_string(),
        ));
    }

    validate_runtime_references(&providers, &raw.runtime)?;

    Ok(AppConfig {
        providers,
        runtime: RuntimePolicy {
            retry_max_attempts: raw.runtime.retry_max_attempts,
            max_concurrency: raw.runtime.max_concurrency,
            rate_limit_per_minute: raw.runtime.rate_limit_per_minute,
            default_provider: raw.runtime.default_provider,
            default_model: raw.runtime.default_model,
        },
    })
}

fn parse_auth_mode(raw: &str) -> Result<AuthMode, ConfigError> {
    match raw.trim().to_lowercase().as_str() {
        "none" => Ok(AuthMode::None),
        "bearer" => Ok(AuthMode::Bearer),
        "browser" => Ok(AuthMode::Browser),
        _ => Err(ConfigError::Invalid(format!("auth_mode invalido: {raw}"))),
    }
}

fn resolve_auth_token(
    provider_name: &str,
    auth_mode: &AuthMode,
    auth_env_var: Option<&str>,
) -> Result<Option<String>, ConfigError> {
    match auth_mode {
        AuthMode::None => {
            if auth_env_var.is_some() {
                return Err(ConfigError::Invalid(format!(
                    "Provider {} com auth_mode none nao deve definir auth_env_var",
                    provider_name
                )));
            }
            Ok(None)
        }
        AuthMode::Browser => {
            if auth_env_var.is_some() {
                return Err(ConfigError::Invalid(format!(
                    "Provider {} com auth_mode browser nao deve definir auth_env_var",
                    provider_name
                )));
            }
            Ok(None)
        }
        AuthMode::Bearer => {
            let var_name = auth_env_var.ok_or_else(|| {
                ConfigError::Invalid(format!(
                    "Provider {} com auth_mode bearer exige auth_env_var",
                    provider_name
                ))
            })?;

            let token = std::env::var(var_name).map_err(|_| {
                ConfigError::Invalid(format!(
                    "Variavel de ambiente {} nao definida para provider {}",
                    var_name, provider_name
                ))
            })?;

            if token.trim().is_empty() {
                return Err(ConfigError::Invalid(format!(
                    "Variavel de ambiente {} esta vazia para provider {}",
                    var_name, provider_name
                )));
            }

            Ok(Some(token))
        }
    }
}

fn validate_runtime_references(
    providers: &[ProviderConfig],
    runtime: &RuntimePolicyFile,
) -> Result<(), ConfigError> {
    let provider = providers
        .iter()
        .find(|p| p.name == runtime.default_provider)
        .ok_or_else(|| {
            ConfigError::Invalid(format!(
                "runtime.default_provider referencia provider inexistente: {}",
                runtime.default_provider
            ))
        })?;

    if !provider
        .models
        .iter()
        .any(|model| model == &runtime.default_model)
    {
        return Err(ConfigError::Invalid(format!(
            "runtime.default_model {} nao existe no provider {}",
            runtime.default_model, runtime.default_provider
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn unique_path() -> PathBuf {
        std::env::temp_dir().join(format!("maestro-config-{}.toml", Uuid::new_v4()))
    }

    fn valid_config_toml(env_var: &str) -> String {
        format!(
            r#"
[[providers]]
name = "ollama"
endpoint = "http://127.0.0.1:11434/v1"
auth_mode = "bearer"
auth_env_var = "{env_var}"
timeout_ms = 5000
models = ["deepseek-coder-v2", "qwen2.5-coder"]

[runtime]
retry_max_attempts = 3
max_concurrency = 4
rate_limit_per_minute = 120
default_provider = "ollama"
default_model = "deepseek-coder-v2"
"#
        )
    }

    #[test]
    fn loads_valid_config() {
        let path = unique_path();
        let env_var = format!("MAESTRO_TOKEN_{}", Uuid::new_v4().as_simple());

        std::env::set_var(&env_var, "token-value");
        let write_result = fs::write(&path, valid_config_toml(&env_var));
        assert!(write_result.is_ok());

        let loaded = ConfigLoader::load(Some(path.clone()));

        assert!(loaded.is_ok());
        let cfg = loaded.ok();
        assert!(cfg.is_some());
        if let Some(config) = cfg {
            assert_eq!(config.providers.len(), 1);
            assert_eq!(config.runtime.default_provider, "ollama");
            assert_eq!(config.runtime.default_model, "deepseek-coder-v2");
            assert_eq!(
                config.providers[0].auth_token,
                Some("token-value".to_string())
            );
        }

        std::env::remove_var(&env_var);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn fails_with_missing_required_field() {
        let path = unique_path();
        let content = r#"
[[providers]]
name = "ollama"
endpoint = "http://127.0.0.1:11434/v1"
auth_mode = "none"
timeout_ms = 5000
models = ["deepseek-coder-v2"]

[runtime]
retry_max_attempts = 3
max_concurrency = 4
rate_limit_per_minute = 120
default_provider = "ollama"
"#;

        let write_result = fs::write(&path, content);
        assert!(write_result.is_ok());

        let loaded = ConfigLoader::load(Some(path.clone()));

        assert!(matches!(loaded, Err(ConfigError::ParseToml(_))));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn fails_with_invalid_type() {
        let path = unique_path();
        let content = r#"
[[providers]]
name = "ollama"
endpoint = "http://127.0.0.1:11434/v1"
auth_mode = "none"
timeout_ms = "fast"
models = ["deepseek-coder-v2"]

[runtime]
retry_max_attempts = 3
max_concurrency = 4
rate_limit_per_minute = 120
default_provider = "ollama"
default_model = "deepseek-coder-v2"
"#;

        let write_result = fs::write(&path, content);
        assert!(write_result.is_ok());

        let loaded = ConfigLoader::load(Some(path.clone()));

        assert!(matches!(loaded, Err(ConfigError::ParseToml(_))));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn fails_with_broken_cross_reference() {
        let path = unique_path();
        let content = r#"
[[providers]]
name = "ollama"
endpoint = "http://127.0.0.1:11434/v1"
auth_mode = "none"
timeout_ms = 5000
models = ["deepseek-coder-v2"]

[runtime]
retry_max_attempts = 3
max_concurrency = 4
rate_limit_per_minute = 120
default_provider = "openai"
default_model = "gpt-4o"
"#;

        let write_result = fs::write(&path, content);
        assert!(write_result.is_ok());

        let loaded = ConfigLoader::load(Some(path.clone()));

        assert!(matches!(
            loaded,
            Err(ConfigError::Invalid(msg)) if msg.contains("default_provider")
        ));

        let _ = fs::remove_file(path);
    }
}
