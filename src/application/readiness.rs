use std::env;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::application::config::ConfigLoader;
use crate::application::markdown_governance::MarkdownGovernance;
use crate::domain::ports::llm_provider::ProviderStatus;
use crate::infrastructure::llm::provider_registry::ProviderRegistry;

#[derive(Debug, Clone)]
pub struct ReadinessItem {
    pub name: String,
    pub passed: bool,
    pub dummy_guide: String,
}

#[derive(Debug, Default)]
pub struct ReadinessState {
    pub items: Vec<ReadinessItem>,
    pub has_config: bool,
    pub config_valid: bool,
    pub has_providers: bool,
    pub provider_reachable: bool,
    pub has_scopes: bool,
    pub has_personas: bool,
    pub has_skills: bool,
    /// SENSE-stage signal: the default provider is reachable, authenticated, and
    /// actually serving its configured model. Populated only by the async
    /// `run_checks_with_probe`; the synchronous `run_checks` leaves it `false`.
    pub model_loaded: bool,
}

impl ReadinessState {
    pub fn is_ready(&self) -> bool {
        self.items.iter().all(|i| i.passed)
    }
}

pub fn run_checks(root: &Path) -> ReadinessState {
    let mut items = Vec::new();

    let dir_ok = env::current_dir().is_ok();
    items.push(ReadinessItem {
        name: "Current directory access".to_string(),
        passed: dir_ok,
        dummy_guide: "How-To: Verify if you have read and execute permissions for the current directory using 'ls -la'.".to_string(),
    });

    let path_ok = env::var("PATH").is_ok();
    items.push(ReadinessItem {
        name: "Environment variables (PATH)".to_string(),
        passed: path_ok,
        dummy_guide: "How-To: Ensure your PATH environment variable is exported correctly in your shell profile (~/.bashrc or ~/.zshrc).".to_string(),
    });

    let maestro_dir = root.join("maestro");
    let config_path = crate::application::config::existing_config_in(&maestro_dir)
        .unwrap_or_else(|| maestro_dir.join(crate::application::config::CONFIG_FILE_NAME));
    let has_config = config_path.exists();
    items.push(ReadinessItem {
        name: "Maestro configuration file".to_string(),
        passed: has_config,
        dummy_guide: "How-To: Create a 'maestro/config.yml' file by running 'maestro init-config' or setting it up manually.".to_string(),
    });

    let mut config_valid = false;
    let mut has_providers = false;
    let mut provider_reachable = false;

    if has_config {
        match ConfigLoader::load(Some(config_path.clone())) {
            Ok(config) => {
                config_valid = true;
                items.push(ReadinessItem {
                    name: "Configuration Content".to_string(),
                    passed: true,
                    dummy_guide: "Configuration is valid.".to_string(),
                });

                has_providers = !config.providers.is_empty();
                items.push(ReadinessItem {
                    name: "Providers Configuration".to_string(),
                    passed: has_providers,
                    dummy_guide: "How-To: Define at least one provider in 'maestro/config.yml' under providers:.".to_string(),
                });

                if has_providers {
                    let default_name = &config.system.default_provider;
                    if let Some(dp) = config.providers.get(default_name) {
                        provider_reachable = endpoint_is_reachable(&dp.endpoint);
                        items.push(ReadinessItem {
                            name: format!("Provider Reachability ({})", default_name),
                            passed: provider_reachable,
                            dummy_guide: format!("How-To: Ensure the provider at '{}' is online and accessible. (e.g. 'ollama serve' or check network).", dp.endpoint),
                        });
                    } else {
                        items.push(ReadinessItem {
                            name: "Provider Reachability".to_string(),
                            passed: false,
                            dummy_guide: "How-To: Ensure 'system.default_provider' matches a defined provider in config.yml.".to_string(),
                        });
                    }
                }
            }
            Err(e) => {
                items.push(ReadinessItem {
                    name: "Configuration Content".to_string(),
                    passed: false,
                    dummy_guide: format!(
                        "How-To: Fix the following error in your config.yml: {}",
                        e
                    ),
                });
            }
        }
    }

    let governance = MarkdownGovernance::new(root);
    let has_scopes = dir_has_markdown(&governance.scopes_dir());
    items.push(ReadinessItem {
        name: "Scopes Directory".to_string(),
        passed: has_scopes,
        dummy_guide: "How-To: Create at least one scope markdown file in 'maestro/scopes/'."
            .to_string(),
    });

    let has_personas = dir_has_markdown(&governance.personas_dir());
    items.push(ReadinessItem {
        name: "Personas Directory".to_string(),
        passed: has_personas,
        dummy_guide: "How-To: Create at least one persona markdown file in 'maestro/personas/'."
            .to_string(),
    });

    let has_skills = skills_has_markdown(&governance.skills_dir());
    items.push(ReadinessItem {
        name: "Skills Directory".to_string(),
        passed: has_skills,
        dummy_guide:
            "How-To: Create at least one skill markdown file in 'maestro/skills/<persona>/'."
                .to_string(),
    });

    ReadinessState {
        items,
        has_config,
        config_valid,
        has_providers,
        provider_reachable,
        has_scopes,
        has_personas,
        has_skills,
        model_loaded: false,
    }
}

/// SENSE stage: resolve the default provider and probe whether its configured
/// model is actually being served.
///
/// Returns `ProviderStatus::Unreachable` when no valid config or default provider
/// can be resolved, so callers can treat it as "no model loaded" without special
/// casing.
pub async fn probe_default_provider(root: &Path) -> ProviderStatus {
    let maestro_dir = root.join("maestro");
    let config_path = match crate::application::config::existing_config_in(&maestro_dir) {
        Some(path) => path,
        None => return ProviderStatus::Unreachable,
    };

    let config = match ConfigLoader::load(Some(config_path)) {
        Ok(config) => config,
        Err(_) => return ProviderStatus::Unreachable,
    };

    let mut registry = ProviderRegistry::new();
    if registry.register_builtin_providers().is_err() {
        return ProviderStatus::Unreachable;
    }
    let resolved = match registry.resolve_default(&config) {
        Ok(resolved) => resolved,
        Err(_) => return ProviderStatus::Unreachable,
    };

    resolved.provider.probe().await
}

/// Async variant of [`run_checks`] that additionally performs the SENSE-stage
/// model-availability probe and records the result in `model_loaded`.
pub async fn run_checks_with_probe(root: &Path) -> ReadinessState {
    let mut state = run_checks(root);
    state.model_loaded = matches!(
        probe_default_provider(root).await,
        ProviderStatus::Available
    );
    state
}

pub fn check_readiness() -> bool {
    let root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let state = run_checks(&root);
    state.is_ready()
}

pub fn print_readiness_failure() {
    let root = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let state = run_checks(&root);
    let failed: Vec<_> = state.items.iter().filter(|r| !r.passed).collect();

    if failed.is_empty() {
        return;
    }

    println!("=== Readiness ===");
    println!("Status: Not Passed\n");

    println!("Passed:");
    for r in state.items.iter().filter(|r| r.passed) {
        println!("  ✅ {}", r.name);
    }

    println!("\nFailed:");
    for f in &failed {
        println!("  ❌ {}", f.name);
        println!("     {}", f.dummy_guide);
    }
    println!("=================");
}

pub fn endpoint_is_reachable(endpoint: &str) -> bool {
    let default_port = if endpoint.starts_with("https://") {
        443
    } else {
        80
    };

    let without_scheme = if let Some(value) = endpoint.strip_prefix("http://") {
        value
    } else if let Some(value) = endpoint.strip_prefix("https://") {
        value
    } else {
        endpoint
    };

    let authority = without_scheme.split('/').next().unwrap_or_default();
    if authority.is_empty() {
        return false;
    }

    let (host, port) = split_host_port(authority, default_port);
    let target = format!("{}:{}", host, port);
    let timeout = Duration::from_millis(800);

    if let Ok(addresses) = target.to_socket_addrs() {
        for addr in addresses {
            if TcpStream::connect_timeout(&addr, timeout).is_ok() {
                return true;
            }
        }
    }

    false
}

fn split_host_port(authority: &str, default_port: u16) -> (String, u16) {
    if let Some(index) = authority.rfind(':') {
        let host = authority[..index].to_string();
        let raw_port = &authority[index + 1..];
        if let Ok(port) = raw_port.parse::<u16>() {
            return (host, port);
        }
    }

    (authority.to_string(), default_port)
}

pub fn dir_has_markdown(path: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("md") {
                return true;
            }
        }
    }
    false
}

pub fn skills_has_markdown(skills_dir: &Path) -> bool {
    if let Ok(persona_dirs) = std::fs::read_dir(skills_dir) {
        for persona_dir in persona_dirs.flatten() {
            let persona_path = persona_dir.path();
            if !persona_path.is_dir() {
                continue;
            }

            if let Ok(skill_files) = std::fs::read_dir(persona_path) {
                for skill_entry in skill_files.flatten() {
                    let skill_path = skill_entry.path();
                    if skill_path.is_file()
                        && skill_path.extension().and_then(|e| e.to_str()) == Some("md")
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Auto-bootstrap configuration if missing.
/// Creates maestro/config.yml with detected provider endpoints.
pub fn auto_bootstrap_config(root: &Path) -> Result<bool, Box<dyn std::error::Error>> {
    let maestro_dir = root.join("maestro");
    std::fs::create_dir_all(&maestro_dir)?;

    if let Some(config_path) = crate::application::config::existing_config_in(&maestro_dir) {
        // Never overwrite a user-owned config. If invalid, provide an actionable error.
        if ConfigLoader::load(Some(config_path.clone())).is_ok() {
            return Ok(false);
        }

        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "existing config is invalid at '{}'; fix or remove it and retry",
                config_path.display()
            ),
        )));
    }

    let config_path = maestro_dir.join(crate::application::config::CONFIG_FILE_NAME);
    let config_content = crate::application::config::DEFAULT_CONFIG_TEMPLATE.to_string();

    std::fs::write(&config_path, config_content)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(prefix: &str) -> PathBuf {
        let unique = format!(
            "{}-{}-{}",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        std::env::temp_dir().join(unique)
    }

    #[test]
    fn auto_bootstrap_creates_valid_yaml_config() {
        let root = temp_root("maestro-readiness-bootstrap");
        let created = std::fs::create_dir_all(&root);
        assert!(created.is_ok());

        let bootstrapped = auto_bootstrap_config(&root);
        assert!(matches!(bootstrapped, Ok(true)));

        let config_path = root.join("maestro").join("config.yml");
        assert!(config_path.exists());
        let loaded = ConfigLoader::load(Some(config_path));
        assert!(loaded.is_ok());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn auto_bootstrap_does_not_overwrite_existing_valid_config() {
        let root = temp_root("maestro-readiness-existing-config");
        let maestro_dir = root.join("maestro");
        let created = std::fs::create_dir_all(&maestro_dir);
        assert!(created.is_ok());

        let config_path = maestro_dir.join("config.yaml");
        let wrote = std::fs::write(
            &config_path,
            crate::application::config::DEFAULT_CONFIG_TEMPLATE,
        );
        assert!(wrote.is_ok());

        let first = auto_bootstrap_config(&root);
        assert!(matches!(first, Ok(false)));

        let loaded = ConfigLoader::load(Some(config_path));
        assert!(loaded.is_ok());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn run_checks_leaves_model_loaded_false() {
        let root = temp_root("maestro-readiness-model-loaded");
        let created = std::fs::create_dir_all(&root);
        assert!(created.is_ok());

        let state = run_checks(&root);
        assert!(!state.model_loaded);

        let _ = std::fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn probe_default_provider_is_unreachable_without_config() {
        let root = temp_root("maestro-readiness-probe-noconfig");
        let created = std::fs::create_dir_all(&root);
        assert!(created.is_ok());

        let status = probe_default_provider(&root).await;
        assert_eq!(status, ProviderStatus::Unreachable);

        let state = run_checks_with_probe(&root).await;
        assert!(!state.model_loaded);

        let _ = std::fs::remove_dir_all(root);
    }
}
