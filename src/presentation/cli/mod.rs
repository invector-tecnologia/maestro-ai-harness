use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::info;

use crate::application::agent_runtime::AgentRuntime;
use crate::application::config::ConfigLoader;
use crate::application::environment::Environment;
use crate::application::markdown_governance::MarkdownGovernance;
use crate::application::persona::PersonaCatalog;
use crate::application::persona_operations::registrations_from_default_personas;
use crate::infrastructure::llm::gemini_adapter::GeminiAdapter;
use crate::infrastructure::llm::provider_registry::ProviderRegistry;
use crate::presentation::tui::{run_tui, OnboardingBootstrap};

#[derive(Debug, Parser)]
#[command(name = "maestro", about = "Maestro Multi-Agent Orchestrator")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Run {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, default_value_t = 200)]
        duration_ms: u64,
    },
    Tui {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Onboarding {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long, default_value = "user")]
        mode: String,
    },
    ValidateConfig {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    ListAgents,
    Doctor {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    ScaffoldMarkdown,
    InitConfig,
    Init {
        project_name: String,
    },
    Logout,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CliOutcome {
    RunCompleted,
    TuiCompleted,
    OnboardingCompleted,
    ConfigValid,
    AgentsListed(Vec<String>),
    DoctorOk,
    ScaffoldDone,
    ConfigInitialized,
    ProjectInitialized,
    LogoutCompleted,
}

pub async fn execute(cli: Cli) -> Result<CliOutcome> {
    let command = cli.command.unwrap_or(Commands::Tui { config: None });

    let is_ready = crate::application::readiness::check_readiness();

    match command {
        Commands::Run {
            config,
            duration_ms,
        } => {
            if !is_ready {
                crate::application::readiness::print_readiness_failure();
                std::process::exit(1);
            }
            let cfg = ConfigLoader::load(config)?;
            let mut registry = ProviderRegistry::new();
            registry.register_builtin_providers()?;
            let resolved = registry.resolve_default(&cfg)?;

            let environment = Arc::new(Environment::new(128));
            let runtime = Arc::new(AgentRuntime::new(Arc::clone(&environment)));
            let registrations = registrations_from_default_personas(resolved.provider)?;
            runtime.start_agents(registrations).await?;
            tokio::time::sleep(std::time::Duration::from_millis(duration_ms)).await;
            runtime.stop_all().await?;

            info!("Run completed");
            Ok(CliOutcome::RunCompleted)
        }
        Commands::Tui { config } => {
            let (environment, runtime) = if let Ok(cfg) = ConfigLoader::load(config) {
                let mut registry = ProviderRegistry::new();
                let _ = registry.register_builtin_providers();
                if let Ok(resolved) = registry.resolve_default(&cfg) {
                    let env = Arc::new(Environment::new(128));
                    let rt = Arc::new(AgentRuntime::new(Arc::clone(&env)));
                    if let Ok(registrations) = registrations_from_default_personas(resolved.provider) {
                        let _ = rt.start_agents(registrations).await;
                    }
                    (Some(env), Some(rt))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            let tui_result = run_tui(
                environment,
                runtime.clone(),
                OnboardingBootstrap::Auto,
            )
            .await;
            if let Some(rt) = runtime {
                let _ = rt.stop_all().await;
            }
            tui_result?;

            Ok(CliOutcome::TuiCompleted)
        }
        Commands::Onboarding { config, mode } => {
            let (environment, runtime) = if let Ok(cfg) = ConfigLoader::load(config) {
                let mut registry = ProviderRegistry::new();
                let _ = registry.register_builtin_providers();
                if let Ok(resolved) = registry.resolve_default(&cfg) {
                    let env = Arc::new(Environment::new(128));
                    let rt = Arc::new(AgentRuntime::new(Arc::clone(&env)));
                    if let Ok(registrations) = registrations_from_default_personas(resolved.provider) {
                        let _ = rt.start_agents(registrations).await;
                    }
                    (Some(env), Some(rt))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            let bootstrap = match mode.to_lowercase().as_str() {
                "user" | "usuario" => OnboardingBootstrap::UserIntro,
                "project" | "projeto" => OnboardingBootstrap::ProjectSetup,
                _ => OnboardingBootstrap::Auto,
            };

            let tui_result =
                run_tui(environment, runtime.clone(), bootstrap).await;
            if let Some(rt) = runtime {
                let _ = rt.stop_all().await;
            }
            tui_result?;

            Ok(CliOutcome::OnboardingCompleted)
        }
        Commands::ValidateConfig { config } => {
            let _ = ConfigLoader::load(config)?;
            info!("Configuration is valid");
            Ok(CliOutcome::ConfigValid)
        }
        Commands::ListAgents => {
            let catalog = PersonaCatalog::default_personas();
            catalog.validate()?;
            let mut names = catalog
                .personas
                .into_iter()
                .map(|persona| persona.name)
                .collect::<Vec<_>>();
            names.sort();

            info!(count = names.len(), "Personas listed");
            Ok(CliOutcome::AgentsListed(names))
        }
        Commands::Doctor { config } => {
            let _ = ConfigLoader::load(config)?;
            let root = std::env::current_dir()?;
            let governance = MarkdownGovernance::new(root);
            governance.ensure_directories()?;
            info!("Doctor check OK");
            Ok(CliOutcome::DoctorOk)
        }
        Commands::ScaffoldMarkdown => {
            let root = std::env::current_dir()?;
            let governance = MarkdownGovernance::new(&root);
            governance.ensure_directories()?;

            scaffold_scope(&governance)?;
            scaffold_personas(&governance)?;
            scaffold_skills(&governance)?;

            info!("Markdown scaffold completed");
            Ok(CliOutcome::ScaffoldDone)
        }
        Commands::InitConfig => {
            let root = std::env::current_dir()?;
            let maestro_dir = root.join("maestro");
            fs::create_dir_all(&maestro_dir)?;
            let config_file = maestro_dir.join("config.toml");
            if !config_file.exists() {
                fs::write(
                    &config_file,
                    crate::application::config::DEFAULT_CONFIG_TEMPLATE,
                )?;
                info!("Configuration generated at {}", config_file.display());
            } else {
                info!("Configuration already exists at {}", config_file.display());
            }
            Ok(CliOutcome::ConfigInitialized)
        }
        Commands::Init { project_name } => {
            let base_dir = std::env::current_dir()?;
            let root = base_dir.join(&project_name);
            fs::create_dir_all(&root)?;

            let maestro_dir = root.join("maestro");
            fs::create_dir_all(&maestro_dir)?;
            let config_file = maestro_dir.join("config.toml");
            if !config_file.exists() {
                fs::write(
                    &config_file,
                    crate::application::config::DEFAULT_CONFIG_TEMPLATE,
                )?;
                info!("Configuration generated at {}", config_file.display());
            } else {
                info!("Configuration already exists at {}", config_file.display());
            }

            let governance = MarkdownGovernance::new(&root);
            governance.ensure_directories()?;

            scaffold_scope(&governance)?;
            scaffold_personas(&governance)?;
            scaffold_skills(&governance)?;

            info!("Project {} initialized", project_name);
            Ok(CliOutcome::ProjectInitialized)
        }
        Commands::Logout => {
            println!("Logging out from external providers...");
            let _ = GeminiAdapter::clear_credentials();
            println!("✅ Logout completed successfully.");
            Ok(CliOutcome::LogoutCompleted)
        }
    }
}

fn scaffold_scope(governance: &MarkdownGovernance) -> Result<()> {
    let file = governance.scopes_dir().join("001-First-Release.md");
    if !file.exists() {
        fs::write(
            file,
            "## Objective\nDefine first release\n\n## Business Scope\nInitial user value\n\n## Deliverables\nInitial backlog\n\n## Acceptance Criteria\nCritical defined\n\n## Dependencies\nNone\n",
        )?;
    }
    Ok(())
}

fn scaffold_personas(governance: &MarkdownGovernance) -> Result<()> {
    let personas = ["product", "engineering", "ux", "devops"];
    for persona in personas {
        let file = governance.personas_dir().join(format!("{persona}.md"));
        if !file.exists() {
            fs::write(
                file,
                "## Responsibility\nDefine responsibility\n\n## Deliverables\nDefine deliverables\n\n## Instructions\nDefine instructions\n\n## Interaction Matrix\nDefine interactions\n\n## Boundaries\nDefine boundaries\n",
            )?;
        }
    }
    Ok(())
}

fn scaffold_skills(governance: &MarkdownGovernance) -> Result<()> {
    let personas = ["product", "engineering", "ux", "devops"];
    for persona in personas {
        let dir = governance.skills_dir().join(persona);
        fs::create_dir_all(&dir)?;
        let file = dir.join("README.md");
        if !file.exists() {
            fs::write(
                file,
                "## Objective\nDescribe objective\n\n## Triggers\nDescribe triggers\n\n## Inputs\nDescribe inputs\n\n## Outputs\nDescribe outputs\n\n## Constraints\nDescribe constraints\n",
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn unique_path() -> PathBuf {
        std::env::temp_dir().join(format!("maestro-cli-{}.toml", Uuid::new_v4()))
    }

    fn write_valid_config(path: &PathBuf) {
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
default_model = "deepseek-coder-v2"
"#;

        let write = fs::write(path, content);
        assert!(write.is_ok());
    }

    #[test]
    fn parses_run_command() {
        let cli = Cli::parse_from(["maestro", "run", "--duration-ms", "50"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Run {
                duration_ms: 50,
                ..
            })
        ));
    }

    #[test]
    fn parses_tui_command() {
        let cli = Cli::parse_from(["maestro", "tui"]);
        assert!(matches!(cli.command, Some(Commands::Tui { .. })));
    }

    #[test]
    fn parses_onboarding_command() {
        let cli = Cli::parse_from(["maestro", "onboarding", "--mode", "projeto"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Onboarding { mode, .. }) if mode == "projeto"
        ));
    }

    #[tokio::test]
    async fn executes_validate_config_command() {
        let path = unique_path();
        write_valid_config(&path);

        let outcome = execute(Cli {
            command: Some(Commands::ValidateConfig {
                config: Some(path.clone()),
            }),
        })
        .await;

        assert!(matches!(outcome, Ok(CliOutcome::ConfigValid)));
        let _ = fs::remove_file(path);
    }

    #[tokio::test]
    async fn executes_list_agents_command() {
        let outcome = execute(Cli {
            command: Some(Commands::ListAgents),
        })
        .await;

        assert!(matches!(
            outcome,
            Ok(CliOutcome::AgentsListed(names))
            if names == vec!["DevOps", "Engineering", "Product", "UX"]
        ));
    }

    #[tokio::test]
    async fn executes_doctor_and_scaffold_markdown_commands() {
        let config_path = unique_path();
        write_valid_config(&config_path);

        let root = std::env::temp_dir().join(format!("maestro-work-{}", Uuid::new_v4()));
        let mkdir = fs::create_dir_all(&root);
        assert!(mkdir.is_ok());

        let old_dir = std::env::current_dir();
        assert!(old_dir.is_ok());
        let old_dir = old_dir.unwrap_or_else(|_| PathBuf::from("."));

        let change = std::env::set_current_dir(&root);
        assert!(change.is_ok());

        let doctor = execute(Cli {
            command: Some(Commands::Doctor {
                config: Some(config_path.clone()),
            }),
        })
        .await;
        assert!(matches!(doctor, Ok(CliOutcome::DoctorOk)));

        let scaffold = execute(Cli {
            command: Some(Commands::ScaffoldMarkdown),
        })
        .await;
        assert!(matches!(scaffold, Ok(CliOutcome::ScaffoldDone)));

        assert!(root.join("maestro").join("scopes").exists());
        assert!(root.join("maestro").join("personas").exists());
        assert!(root.join("maestro").join("skills").exists());

        let _ = std::env::set_current_dir(&old_dir);
        let _ = fs::remove_file(config_path);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn parses_init_config_command() {
        let cli = Cli::parse_from(["maestro", "init-config"]);
        assert!(matches!(cli.command, Some(Commands::InitConfig)));
    }

    #[test]
    fn parses_init_command() {
        let cli = Cli::parse_from(["maestro", "init", "meu-projeto"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Init { project_name }) if project_name == "meu-projeto"
        ));
    }

    #[test]
    fn parses_logout_command() {
        let cli = Cli::parse_from(["maestro", "logout"]);
        assert!(matches!(cli.command, Some(Commands::Logout)));
    }

    #[test]
    fn parses_no_command_as_none() {
        let cli = Cli::parse_from(["maestro"]);
        assert!(cli.command.is_none());
    }
}
