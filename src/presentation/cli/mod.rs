use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use tracing::{info, warn};

use crate::application::agent_runtime::AgentRuntime;
use crate::application::config::ConfigLoader;
use crate::application::environment::Environment;
use crate::application::markdown_governance::MarkdownGovernance;
use crate::application::persona::PersonaCatalog;
use crate::application::persona_operations::registrations_from_default_personas;
use crate::application::rag::RagService;
use crate::domain::ports::rag::RagEmbedder;
use crate::infrastructure::llm::gemini_adapter::GeminiAdapter;
use crate::infrastructure::llm::provider_registry::ProviderRegistry;
use crate::infrastructure::rag::local_hybrid_index::LocalHybridIndex;
use crate::infrastructure::rag::ollama_embedder::OllamaEmbedder;
use crate::presentation::tui::{run_tui, OnboardingBootstrap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OnboardingMode {
    Fast,
    Detailed,
}

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
        #[arg(long, value_enum, default_value_t = OnboardingMode::Detailed)]
        mode: OnboardingMode,
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
        #[arg(long, action = ArgAction::SetTrue)]
        no_tui: bool,
    },
    Logout,
    Rag {
        #[command(subcommand)]
        command: RagCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum RagCommands {
    Ingest {
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long, default_value_t = 900)]
        chunk_size_chars: usize,
    },
    Query {
        question: String,
        #[arg(long, default_value_t = 8)]
        top_k: usize,
    },
    Eval {
        #[arg(long, default_value_t = 8)]
        top_k: usize,
    },
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
    RagIngested {
        documents: usize,
        chunks: usize,
    },
    RagQueryCompleted {
        citations: usize,
    },
    RagEvalCompleted {
        cases_total: usize,
        baseline_cases_passed: usize,
        enhanced_cases_passed: usize,
    },
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
            run_tui_with_runtime(config, OnboardingBootstrap::Detailed).await?;

            Ok(CliOutcome::TuiCompleted)
        }
        Commands::Onboarding { config, mode } => {
            let bootstrap = match mode {
                OnboardingMode::Fast => OnboardingBootstrap::Fast,
                OnboardingMode::Detailed => OnboardingBootstrap::Detailed,
            };

            run_tui_with_runtime(config, bootstrap).await?;

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
        Commands::Init {
            project_name,
            no_tui,
        } => {
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

            if !no_tui {
                let old_dir = std::env::current_dir()?;
                std::env::set_current_dir(&root)?;

                let tui_result = run_tui_with_runtime(None, OnboardingBootstrap::Detailed).await;
                let restore_result = std::env::set_current_dir(&old_dir);

                if let Err(err) = restore_result {
                    warn!(
                        error = %err,
                        "failed to restore original working directory after init"
                    );
                }

                tui_result?;
            }

            Ok(CliOutcome::ProjectInitialized)
        }
        Commands::Logout => {
            println!("Logging out from external providers...");
            let _ = GeminiAdapter::clear_credentials();
            println!("✅ Logout completed successfully.");
            Ok(CliOutcome::LogoutCompleted)
        }
        Commands::Rag { command } => {
            let root = std::env::current_dir()?;
            let local_index = Arc::new(LocalHybridIndex::new(&root));
            let embedder = build_rag_embedder().await;
            let rag = RagService::new_with_options(
                local_index.clone(),
                local_index.clone(),
                local_index.clone(),
                embedder,
                root.join("maestro").join("rag"),
            );

            match command {
                RagCommands::Ingest {
                    root,
                    chunk_size_chars,
                } => {
                    let corpus_root = match root {
                        Some(path) => path,
                        None => std::env::current_dir()?,
                    };

                    let default_paths = vec![
                        corpus_root.join("docs"),
                        corpus_root.join("src"),
                        corpus_root.join("README.md"),
                        corpus_root.join("maestro").join("config.toml"),
                    ];

                    let report = rag.ingest_paths(default_paths, chunk_size_chars).await?;

                    info!(
                        docs = report.documents_indexed,
                        chunks = report.chunks_indexed,
                        index = %local_index.index_path().display(),
                        "RAG ingestion completed"
                    );

                    Ok(CliOutcome::RagIngested {
                        documents: report.documents_indexed,
                        chunks: report.chunks_indexed,
                    })
                }
                RagCommands::Query { question, top_k } => {
                    let answer = rag.query(&question, top_k).await?;

                    info!(
                        citations = answer.citations.len(),
                        response = %answer.answer,
                        "RAG query completed"
                    );

                    Ok(CliOutcome::RagQueryCompleted {
                        citations: answer.citations.len(),
                    })
                }
                RagCommands::Eval { top_k } => {
                    let report = rag.evaluate(top_k).await?;
                    info!(
                        total = report.cases_total,
                        baseline_passed = report.baseline_cases_passed,
                        enhanced_passed = report.enhanced_cases_passed,
                        baseline_avg = report.average_baseline_score,
                        enhanced_avg = report.average_enhanced_score,
                        report_path = %report.report_path,
                        "RAG evaluation completed"
                    );

                    Ok(CliOutcome::RagEvalCompleted {
                        cases_total: report.cases_total,
                        baseline_cases_passed: report.baseline_cases_passed,
                        enhanced_cases_passed: report.enhanced_cases_passed,
                    })
                }
            }
        }
    }
}

async fn run_tui_with_runtime(
    config: Option<PathBuf>,
    bootstrap: OnboardingBootstrap,
) -> Result<()> {
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

    let tui_result = run_tui(environment, runtime.clone(), bootstrap).await;
    if let Some(rt) = runtime {
        let _ = rt.stop_all().await;
    }
    tui_result?;
    Ok(())
}

async fn build_rag_embedder() -> Option<Arc<dyn RagEmbedder>> {
    let config = match ConfigLoader::load(None) {
        Ok(cfg) => cfg,
        Err(_) => return None,
    };

    let provider = match config
        .providers
        .iter()
        .find(|provider| provider.name == config.runtime.default_provider)
    {
        Some(value) => value,
        None => return None,
    };

    let model = match provider
        .models
        .iter()
        .find(|m| *m == &config.runtime.default_model)
    {
        Some(value) => value,
        None => match provider.models.first() {
            Some(fallback) => fallback,
            None => return None,
        },
    };

    match OllamaEmbedder::new(&provider.endpoint, model, provider.timeout_ms) {
        Ok(embedder) => Some(Arc::new(embedder)),
        Err(err) => {
            warn!(error = %err, "failed to initialize rag embedder; using lexical-only retrieval");
            None
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
        let cli = Cli::parse_from(["maestro", "onboarding", "--mode", "fast"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Onboarding {
                mode: OnboardingMode::Fast,
                ..
            })
        ));
    }

    #[test]
    fn parses_detailed_onboarding_command_by_default() {
        let cli = Cli::parse_from(["maestro", "onboarding"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Onboarding {
                mode: OnboardingMode::Detailed,
                ..
            })
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
            Some(Commands::Init {
                project_name,
                no_tui: false
            }) if project_name == "meu-projeto"
        ));
    }

    #[test]
    fn parses_init_command_with_no_tui() {
        let cli = Cli::parse_from(["maestro", "init", "meu-projeto", "--no-tui"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Init {
                project_name,
                no_tui: true
            }) if project_name == "meu-projeto"
        ));
    }

    #[test]
    fn parses_logout_command() {
        let cli = Cli::parse_from(["maestro", "logout"]);
        assert!(matches!(cli.command, Some(Commands::Logout)));
    }

    #[test]
    fn parses_rag_ingest_command() {
        let cli = Cli::parse_from(["maestro", "rag", "ingest", "--chunk-size-chars", "700"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Rag {
                command: RagCommands::Ingest {
                    chunk_size_chars: 700,
                    ..
                }
            })
        ));
    }

    #[test]
    fn parses_rag_query_command() {
        let cli = Cli::parse_from(["maestro", "rag", "query", "What is KV cache?"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Rag {
                command: RagCommands::Query { .. }
            })
        ));
    }

    #[test]
    fn parses_rag_eval_command() {
        let cli = Cli::parse_from(["maestro", "rag", "eval", "--top-k", "6"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Rag {
                command: RagCommands::Eval { top_k: 6 }
            })
        ));
    }

    #[test]
    fn parses_no_command_as_none() {
        let cli = Cli::parse_from(["maestro"]);
        assert!(cli.command.is_none());
    }
}
