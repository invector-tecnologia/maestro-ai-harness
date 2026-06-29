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
use crate::application::persona_operations::{
    registrations_from_default_personas, registrations_from_governance,
    registrations_from_selected_personas,
};
use crate::application::project_deps::{
    ProjectDependencyCheck, ProjectDepsCheckReport, ProjectDepsConfig,
    DEFAULT_PROJECT_DEPS_TEMPLATE,
};
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
    Interview {
        #[arg(long)]
        config: Option<PathBuf>,
    },
    Directives {
        #[arg(long)]
        config: Option<PathBuf>,
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
    Deps {
        #[command(subcommand)]
        command: DepsCommands,
    },
    Rag {
        #[command(subcommand)]
        command: RagCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum DepsCommands {
    Check {
        #[arg(long, value_enum, default_value_t = DepsScope::All)]
        scope: DepsScope,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        deps_file: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DepsScope {
    Harness,
    Project,
    All,
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
    InterviewCompleted,
    DirectivesLaunched,
    ConfigValid,
    AgentsListed(Vec<String>),
    DoctorOk,
    ScaffoldDone,
    ConfigInitialized,
    ProjectInitialized,
    LogoutCompleted,
    DepsChecked {
        harness_ready: bool,
        project_ready: bool,
    },
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
            let router = registry.build_model_router(&cfg)?;

            let environment = Arc::new(Environment::new(128));
            let runtime = Arc::new(AgentRuntime::new(Arc::clone(&environment)));
            let registrations = registrations_from_default_personas(&router)?;
            for registration in &registrations {
                info!(
                    agent = %registration.name,
                    route = %router.label_for(&registration.name).descriptor(),
                    "agent model route"
                );
            }
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
        Commands::Interview { config } => {
            run_tui_with_runtime(config, OnboardingBootstrap::InitInterview).await?;

            Ok(CliOutcome::InterviewCompleted)
        }
        Commands::Directives { config } => {
            run_tui_with_runtime(config, OnboardingBootstrap::DirectiveGovernance).await?;

            Ok(CliOutcome::DirectivesLaunched)
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
            scaffold_project_deps(&root)?;

            info!("Markdown scaffold completed");
            Ok(CliOutcome::ScaffoldDone)
        }
        Commands::InitConfig => {
            let root = std::env::current_dir()?;
            let maestro_dir = root.join("maestro");
            fs::create_dir_all(&maestro_dir)?;
            let config_file = maestro_dir.join("config.yml");
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
            let config_file = maestro_dir.join("config.yml");
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
            scaffold_project_deps(&root)?;

            info!("Project {} initialized", project_name);

            if !no_tui {
                let old_dir = std::env::current_dir()?;
                std::env::set_current_dir(&root)?;

                let tui_result =
                    run_tui_with_runtime(None, OnboardingBootstrap::InitInterview).await;
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
        Commands::Deps { command } => match command {
            DepsCommands::Check {
                scope,
                config,
                deps_file,
            } => {
                let mut harness_ready = true;
                let mut project_ready = true;

                if matches!(scope, DepsScope::Harness | DepsScope::All) {
                    harness_ready = check_harness_dependencies(config.as_ref())?;
                }

                if matches!(scope, DepsScope::Project | DepsScope::All) {
                    let report = check_project_dependencies(deps_file)?;
                    project_ready = report.all_required_passed();
                    print_project_dependency_report(&report);
                }

                Ok(CliOutcome::DepsChecked {
                    harness_ready,
                    project_ready,
                })
            }
        },
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

                    let maestro_dir = corpus_root.join("maestro");
                    let config_corpus_path =
                        crate::application::config::existing_config_in(&maestro_dir)
                            .unwrap_or_else(|| {
                                maestro_dir.join(crate::application::config::CONFIG_FILE_NAME)
                            });
                    let default_paths = vec![
                        corpus_root.join("docs"),
                        corpus_root.join("src"),
                        corpus_root.join("README.md"),
                        config_corpus_path,
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

fn check_harness_dependencies(config: Option<&PathBuf>) -> Result<bool> {
    let cfg = ConfigLoader::load(config.cloned())?;
    let mut registry = ProviderRegistry::new();
    registry.register_builtin_providers()?;
    let _ = registry.build_model_router(&cfg)?;

    let root = std::env::current_dir()?;
    let readiness = crate::application::readiness::run_checks(&root);

    let failed = readiness
        .items
        .iter()
        .filter(|item| !item.passed)
        .collect::<Vec<_>>();

    if failed.is_empty() {
        println!("Harness dependencies: ✅ ready");
        return Ok(true);
    }

    println!("Harness dependencies: ❌ not ready");
    for item in failed {
        println!("  - {}", item.name);
        println!("    {}", item.dummy_guide);
    }

    Ok(false)
}

fn check_project_dependencies(deps_file: Option<PathBuf>) -> Result<ProjectDepsCheckReport> {
    let config = ProjectDepsConfig::load(deps_file)?;
    let mut checks = Vec::new();

    for dep in config.dependencies {
        let status = std::process::Command::new("sh")
            .arg("-lc")
            .arg(&dep.check_command)
            .status();

        let passed = match status {
            Ok(exit) => exit.success(),
            Err(_) => false,
        };

        checks.push(ProjectDependencyCheck {
            name: dep.name,
            passed,
            required: dep.required,
            install_hint: dep.install_hint,
        });
    }

    Ok(ProjectDepsCheckReport { checks })
}

fn print_project_dependency_report(report: &ProjectDepsCheckReport) {
    println!("Project dependencies:");
    for check in &report.checks {
        let status = if check.passed { "✅" } else { "❌" };
        let required = if check.required {
            "required"
        } else {
            "optional"
        };
        println!("  {} {} ({})", status, check.name, required);
        if !check.passed {
            if let Some(hint) = &check.install_hint {
                println!("     {}", hint);
            }
        }
    }

    if report.all_required_passed() {
        println!("Project dependencies: ✅ required checks passed");
    } else {
        println!("Project dependencies: ❌ required checks failed");
    }
}

async fn run_tui_with_runtime(
    config: Option<PathBuf>,
    bootstrap: OnboardingBootstrap,
) -> Result<()> {
    let environment = Arc::new(Environment::new(128));
    let mut runtime: Option<Arc<AgentRuntime>> = None;
    let mut handoff_router: Option<crate::application::model_router::ModelRouter> = None;

    match ConfigLoader::load(config) {
        Ok(cfg) => {
            let mut registry = ProviderRegistry::new();
            if let Err(error) = registry.register_builtin_providers() {
                let _ = environment
                    .publish(crate::domain::models::message::Message::new(
                        "system".to_string(),
                        format!("⚠️ Failed to register builtin LLM providers: {error}"),
                        None,
                    ))
                    .await;
            }

            match registry.build_model_router(&cfg) {
                Ok(router) => {
                    let default_label = router.default_label().clone();
                    handoff_router = Some(router.clone());
                    if let Err(error) = probe_active_default_model(
                        router.default_provider().as_ref(),
                        &default_label.provider,
                        &default_label.model,
                    )
                    .await
                    {
                        let _ = environment
                            .publish(crate::domain::models::message::Message::new(
                                "system".to_string(),
                                format!("⚠️ Startup check failed (active model): {error}"),
                                None,
                            ))
                            .await;
                        let tui_result = run_tui(
                            Some(Arc::clone(&environment)),
                            runtime.clone(),
                            bootstrap,
                            handoff_router.clone(),
                        )
                        .await;
                        if let Some(rt) = runtime {
                            let _ = rt.stop_all().await;
                        }
                        tui_result?;
                        return Ok(());
                    }

                    let rt = Arc::new(AgentRuntime::new(Arc::clone(&environment)));
                    if matches!(bootstrap, OnboardingBootstrap::InitInterview) {
                        match registrations_from_selected_personas(&router, &["Maestro"]) {
                            Ok(registrations) => {
                                if let Err(error) = rt.start_agents(registrations).await {
                                    let _ = environment
                                        .publish(crate::domain::models::message::Message::new(
                                            "system".to_string(),
                                            format!("⚠️ Failed to start runtime personas: {error}"),
                                            None,
                                        ))
                                        .await;
                                } else {
                                    runtime = Some(rt);
                                }
                            }
                            Err(error) => {
                                let _ = environment
                                    .publish(crate::domain::models::message::Message::new(
                                        "system".to_string(),
                                        format!("⚠️ Invalid default personas: {error}"),
                                        None,
                                    ))
                                    .await;
                            }
                        }
                    } else {
                        // Workspace monitor: resolve the governed persona catalog so Core
                        // Mode edits drive the live agent set, then orchestrate the agents
                        // sequentially per user prompt instead of parallel broadcast.
                        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                        let governance = MarkdownGovernance::new(root);
                        let registrations = registrations_from_governance(&router, &governance);
                        for registration in &registrations {
                            info!(
                                agent = %registration.name,
                                route = %router.label_for(&registration.name).descriptor(),
                                "agent model route"
                            );
                        }
                        rt.set_sequential_pipeline(registrations).await;
                        runtime = Some(rt);
                    }
                }
                Err(error) => {
                    let _ = environment
                        .publish(crate::domain::models::message::Message::new(
                            "system".to_string(),
                            format!("⚠️ LLM provider setup failed: {error}"),
                            None,
                        ))
                        .await;
                }
            }
        }
        Err(error) => {
            let _ = environment
                .publish(crate::domain::models::message::Message::new(
                    "system".to_string(),
                    format!("⚠️ Could not load config: {error}"),
                    None,
                ))
                .await;
        }
    }

    let tui_result = run_tui(
        Some(Arc::clone(&environment)),
        runtime.clone(),
        bootstrap,
        handoff_router,
    )
    .await;
    if let Some(rt) = runtime {
        let _ = rt.stop_all().await;
    }
    tui_result?;
    Ok(())
}

async fn probe_active_default_model(
    provider: &dyn crate::domain::ports::llm_provider::LlmProvider,
    provider_name: &str,
    model_name: &str,
) -> Result<()> {
    let probe_prompt = "Reply with the single word: awake";
    let response = provider.text_only(probe_prompt).await.map_err(|error| {
        anyhow::anyhow!(
            "provider '{}' model '{}' did not answer probe: {}",
            provider_name,
            model_name,
            error
        )
    })?;

    if response.trim().is_empty() {
        return Err(anyhow::anyhow!(
            "provider '{}' model '{}' returned an empty probe response",
            provider_name,
            model_name
        ));
    }

    Ok(())
}

async fn build_rag_embedder() -> Option<Arc<dyn RagEmbedder>> {
    let config = match ConfigLoader::load(None) {
        Ok(cfg) => cfg,
        Err(_) => return None,
    };

    let provider = match config.providers.get(&config.system.default_provider) {
        Some(value) => value,
        None => return None,
    };

    let model = match provider
        .models
        .iter()
        .find(|m| m.name == config.system.default_model)
    {
        Some(value) => &value.name,
        None => match provider.models.first() {
            Some(fallback) => &fallback.name,
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
    // Emit the canonical persona schema directly from the runtime catalog so the
    // scaffolded files, the Architect Mode editor, and the live agents share one source
    // of truth.
    for persona in PersonaCatalog::default_personas().personas {
        let slug = persona.name.to_lowercase().replace(' ', "-");
        let file = governance.personas_dir().join(format!("{slug}.md"));
        if !file.exists() {
            fs::write(file, persona.to_markdown())?;
        }
    }
    Ok(())
}

fn scaffold_skills(governance: &MarkdownGovernance) -> Result<()> {
    let personas = [
        (
            "maestro",
            "README.md",
            "## Objective\nGovern directive orchestration and prompt effectiveness\n\n## Triggers\nDirective create/update requests and cross-persona conflicts\n\n## Inputs\nProject intent, constraints, and quality evidence\n\n## Outputs\nApproved directives and handoff decisions\n\n## Constraints\nPersona is immutable and cannot be modified\n",
        ),
        (
            "project-manager",
            "delivery-planning.md",
            "## Objective\nPlan milestone sequencing and launch readiness\n\n## Triggers\nNew scope proposals or delivery risk changes\n\n## Inputs\nBusiness goals, dependencies, and timeline constraints\n\n## Outputs\nPrioritized milestone plan and acceptance checklist\n\n## Constraints\nMust align with Maestro governance decisions\n",
        ),
        (
            "quality-assurance",
            "quality-gate-design.md",
            "## Objective\nBuild risk-based quality gates for each increment\n\n## Triggers\nImplementation updates and release-candidate preparation\n\n## Inputs\nAcceptance criteria, test evidence, and defect trends\n\n## Outputs\nGo/no-go recommendation with traceable evidence\n\n## Constraints\nCannot approve release with missing critical evidence\n",
        ),
        (
            "user-experience",
            "journey-validation.md",
            "## Objective\nValidate usability and reduce interaction friction\n\n## Triggers\nFeature proposals, UX regressions, or onboarding pain points\n\n## Inputs\nUser journeys, feedback, and usage observations\n\n## Outputs\nExperience recommendations and updated interaction guidance\n\n## Constraints\nMust keep changes aligned to approved scope priorities\n",
        ),
        (
            "software-engineer",
            "language-agnostic-engineering.md",
            "## Objective\nDeliver maintainable software using language-agnostic practices\n\n## Triggers\nNew implementation tasks, refactoring, or architecture debt\n\n## Inputs\nScope directives, architecture constraints, and quality gates\n\n## Outputs\nWorking increments with tests and observability hooks\n\n## Constraints\nAvoid language-specific lock-in in general engineering guidance\n",
        ),
    ];
    for (persona, file_name, content) in personas {
        let dir = governance.skills_dir().join(persona);
        fs::create_dir_all(&dir)?;
        let file = dir.join(file_name);
        if !file.exists() {
            fs::write(file, content)?;
        }
    }
    Ok(())
}

fn scaffold_project_deps(root: &std::path::Path) -> Result<()> {
    let path = root.join("maestro").join("project-deps.yml");
    if !path.exists() {
        std::fs::write(path, DEFAULT_PROJECT_DEPS_TEMPLATE)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};

    use super::*;
    use uuid::Uuid;

    fn cwd_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn unique_path() -> PathBuf {
        std::env::temp_dir().join(format!("maestro-cli-{}.toml", Uuid::new_v4()))
    }

    fn write_valid_config(path: &PathBuf) {
        let content = "system:\n  default_provider: \"ollama\"\n  default_model: \"mistral\"\n  max_concurrency: 4\n  rate_limit_per_minute: 120\n  retry_max_attempts: 3\nproviders:\n  ollama:\n    kind: \"ollama\"\n    endpoint: \"http://localhost:11434/v1\"\n    auth_mode: \"none\"\n    timeout_ms: 5000\n    models:\n      - name: \"mistral\"\n        context_window: 32000\n    capabilities:\n      supports_tools: false\n      supports_streaming: true\n      supports_json_mode: false\n      supports_reasoning_controls: false\n      max_context_tokens: 32000\n";

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
    fn parses_interview_command() {
        let cli = Cli::parse_from(["maestro", "interview"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Interview { config: None })
        ));
    }

    #[test]
    fn parses_directives_command() {
        let cli = Cli::parse_from(["maestro", "directives"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Directives { config: None })
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
            if names == vec![
                "Maestro",
                "Project Manager",
                "Quality Assurance",
                "Software Engineer",
                "User Experience"
            ]
        ));
    }

    #[test]
    fn executes_doctor_and_scaffold_markdown_commands() {
        let lock = cwd_lock();
        let guard = lock.lock();
        assert!(guard.is_ok());
        let _guard = guard.unwrap_or_else(|poisoned| poisoned.into_inner());

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        assert!(runtime.is_ok());
        let runtime = runtime.unwrap_or_else(|error| {
            panic!("failed to build tokio runtime for test: {}", error);
        });

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

        let doctor = runtime.block_on(execute(Cli {
            command: Some(Commands::Doctor {
                config: Some(config_path.clone()),
            }),
        }));
        assert!(matches!(doctor, Ok(CliOutcome::DoctorOk)));

        let scaffold = runtime.block_on(execute(Cli {
            command: Some(Commands::ScaffoldMarkdown),
        }));
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
    fn parses_deps_check_command() {
        let cli = Cli::parse_from(["maestro", "deps", "check", "--scope", "project"]);
        assert!(matches!(
            cli.command,
            Some(Commands::Deps {
                command: DepsCommands::Check {
                    scope: DepsScope::Project,
                    ..
                }
            })
        ));
    }

    #[test]
    fn executes_deps_check_project_command() {
        let lock = cwd_lock();
        let guard = lock.lock();
        assert!(guard.is_ok());
        let _guard = guard.unwrap_or_else(|poisoned| poisoned.into_inner());

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        assert!(runtime.is_ok());
        let runtime = runtime.unwrap_or_else(|error| {
            panic!("failed to build tokio runtime for test: {}", error);
        });

        let root = std::env::temp_dir().join(format!("maestro-deps-{}", Uuid::new_v4()));
        assert!(std::fs::create_dir_all(root.join("maestro")).is_ok());

        let deps_path = root.join("maestro").join("project-deps.yml");
        let content = "dependencies:\n  - name: shell\n    check_command: \"command -v sh >/dev/null 2>&1\"\n    required: true\n";
        assert!(std::fs::write(&deps_path, content).is_ok());

        let old = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        assert!(std::env::set_current_dir(&root).is_ok());

        let outcome = runtime.block_on(execute(Cli {
            command: Some(Commands::Deps {
                command: DepsCommands::Check {
                    scope: DepsScope::Project,
                    config: None,
                    deps_file: None,
                },
            }),
        }));

        assert!(matches!(
            outcome,
            Ok(CliOutcome::DepsChecked {
                harness_ready: true,
                project_ready: true
            })
        ));

        let _ = std::env::set_current_dir(old);
        let _ = std::fs::remove_dir_all(root);
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

    #[test]
    fn shipped_default_skills_and_scope_pass_governance_validation() {
        let root = std::env::temp_dir().join(format!("maestro-scaffold-{}", Uuid::new_v4()));
        let governance = MarkdownGovernance::new(&root);
        governance
            .ensure_directories()
            .expect("ensure governance directories");
        scaffold_personas(&governance).expect("scaffold personas");
        scaffold_skills(&governance).expect("scaffold skills");
        scaffold_scope(&governance).expect("scaffold scope");

        // Every non-Maestro persona ships a skill that passes governance validation.
        let mut validated_skills = 0_usize;
        for persona_file in governance.list_personas().expect("list personas") {
            let slug = persona_file
                .strip_suffix(".md")
                .unwrap_or(&persona_file)
                .to_string();
            if slug.eq_ignore_ascii_case("maestro") {
                // Maestro is immutable: governance rejects skill mutation by design.
                continue;
            }
            for skill_file in governance.list_skills(&slug).expect("list skills") {
                let path = governance.skills_dir().join(&slug).join(&skill_file);
                let content = governance.read_document(&path).expect("read skill");
                let result = governance.validate_skill_document(&slug, &skill_file, &content);
                assert!(
                    result.is_ok(),
                    "skill {}/{} must validate: {:?}",
                    slug,
                    skill_file,
                    result
                );
                validated_skills += 1;
            }
        }
        assert!(
            validated_skills >= 4,
            "expected at least four worker skills, found {}",
            validated_skills
        );

        // The default scope ships with the canonical schema; validate its content
        // in a fresh empty workspace so the sequence check treats it as first.
        let scopes = governance.list_scopes().expect("list scopes");
        assert!(!scopes.is_empty(), "expected a default scope");
        let fresh_root = std::env::temp_dir().join(format!("maestro-scope-{}", Uuid::new_v4()));
        let fresh = MarkdownGovernance::new(&fresh_root);
        fresh
            .ensure_directories()
            .expect("ensure fresh directories");
        for scope_file in scopes {
            let path = governance.scopes_dir().join(&scope_file);
            let content = governance.read_document(&path).expect("read scope");
            let result = fresh.validate_scope_document(&scope_file, &content);
            assert!(
                result.is_ok(),
                "scope {} must validate: {:?}",
                scope_file,
                result
            );
        }

        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(fresh_root);
    }
}
