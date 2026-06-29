use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::io::{self, Stdout};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use crossterm::cursor::{DisableBlinking, EnableBlinking};
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use futures_util::StreamExt;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table};
use ratatui::{Frame, Terminal};
use tui_big_text::{BigText, PixelSize};
use uuid::Uuid;

use crate::application::agent_runtime::{AgentHealth, AgentRuntime};
use crate::application::config::DEFAULT_CONFIG_TEMPLATE;
use crate::application::environment::Environment;
use crate::application::markdown_governance::{MarkdownGovernance, MAESTRO_PERSONA_FILE};
use crate::application::project_deps::{ProjectDepsConfig, DEFAULT_PROJECT_DEPS_TEMPLATE};
use crate::domain::models::message::Message;
use crate::infrastructure::llm::gemini_adapter::GeminiAdapter;

#[derive(Debug, Clone)]
pub struct AgentView {
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UIMode {
    #[default]
    Workspace,
    HelpMenu,
    /// Architect Mode: the directive-governance picker (select stage) where
    /// personas, persona skills, and scopes are chosen for authoring.
    Architect,
    Interview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum PanelFocus {
    #[default]
    Input,
    Orchestration,
    AgentActivity,
    Readiness,
}

impl PanelFocus {
    /// Deterministic Workspace focus flow: input -> orchestration -> agent
    /// activity -> readiness/actions, then back to input.
    fn next(self) -> Self {
        match self {
            PanelFocus::Input => PanelFocus::Orchestration,
            PanelFocus::Orchestration => PanelFocus::AgentActivity,
            PanelFocus::AgentActivity => PanelFocus::Readiness,
            PanelFocus::Readiness => PanelFocus::Input,
        }
    }

    /// Human-readable role label for the focused panel.
    fn role_label(self) -> &'static str {
        match self {
            PanelFocus::Input => "Input",
            PanelFocus::Orchestration => "Orchestration",
            PanelFocus::AgentActivity => "Agent Activity",
            PanelFocus::Readiness => "Readiness / Actions",
        }
    }
}

/// Border style for a Workspace panel, highlighted when it currently holds focus.
fn panel_border_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(218, 165, 32))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReadinessAction {
    CreateConfigTemplate,
    CreateProjectDepsTemplate,
    RemediateProjectDependency {
        dependency: String,
        install_hint: Option<String>,
    },
    OpenConfigHint,
    ConfigureProviders,
    StartProvider,
    CreateScope,
    CreatePersona,
    CreateSkill,
}

impl ReadinessAction {
    fn label(&self) -> String {
        match self {
            ReadinessAction::CreateConfigTemplate => "Create config template".to_string(),
            ReadinessAction::CreateProjectDepsTemplate => {
                "Create project deps template".to_string()
            }
            ReadinessAction::RemediateProjectDependency { dependency, .. } => {
                format!("Fix required dependency: {dependency}")
            }
            ReadinessAction::OpenConfigHint => "Open config guidance".to_string(),
            ReadinessAction::ConfigureProviders => "Configure providers".to_string(),
            ReadinessAction::StartProvider => "Start provider".to_string(),
            ReadinessAction::CreateScope => "Create scope".to_string(),
            ReadinessAction::CreatePersona => "Create persona".to_string(),
            ReadinessAction::CreateSkill => "Create skill".to_string(),
        }
    }
}

#[derive(Debug, Default)]
pub struct TuiApp {
    pub agents: Vec<AgentView>,
    pub logs: Vec<String>,
    pub input: String,
    pub mode: UIMode,
    readiness: crate::application::readiness::ReadinessState,
    dependency_domains: DependencyDomainsState,
    focus: PanelFocus,
    readiness_selected_action: usize,
    wizard: Option<CreationWizard>,
    animation_frame: usize,
    play_bell: bool,
    highlight_until: HashMap<String, usize>,
    pub show_debug: bool,
    // Interview mode fields
    interview_session:
        Option<Arc<tokio::sync::RwLock<crate::application::interview_bot::InterviewSession>>>,
    interview_bot: Option<Arc<crate::application::interview_bot::InterviewBot>>,
    #[allow(dead_code)]
    maestro_message_id: Option<Uuid>,
    approval_modal_visible: bool,
    last_runtime_event_count: usize,
    // Architect Mode directives picker
    architect_picker: Option<ArchitectPicker>,
}

#[derive(Debug, Clone, Default)]
struct DependencyDomainsState {
    project_manifest_found: bool,
    project_manifest_valid: bool,
    project_required_checks_passed: bool,
    project_failed_required: Vec<String>,
    project_failed_required_hints: Vec<(String, Option<String>)>,
    project_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingBootstrap {
    Fast,
    Detailed,
    InitInterview,
    DirectiveGovernance,
}

fn should_enter_interview(bootstrap: OnboardingBootstrap, readiness_ready: bool) -> bool {
    match bootstrap {
        OnboardingBootstrap::InitInterview => true,
        OnboardingBootstrap::Detailed => !readiness_ready,
        OnboardingBootstrap::Fast => false,
        OnboardingBootstrap::DirectiveGovernance => false,
    }
}

enum UserAction {
    SubmitCommand(String),
    CompleteWizard(WizardSubmission),
    RunReadinessAction(ReadinessAction),
    ProcessInterviewAnswer(String),
    StartDirectiveAuthoring {
        target: crate::application::interview_bot::DirectiveTarget,
        operation: crate::application::interview_bot::DirectiveOperation,
        file_name: String,
    },
    ManageProjectDeps,
    ApproveInterviewProposals,
    RejectInterviewProposals,
    Quit,
    Logout,
}

pub async fn run_tui(
    environment: Option<Arc<Environment>>,
    runtime: Option<Arc<AgentRuntime>>,
    _bootstrap: OnboardingBootstrap,
) -> Result<()> {
    let mut terminal = setup_terminal()?;
    let mut app = TuiApp {
        show_debug: true,
        ..Default::default()
    };
    app.logs
        .push("🚀 Welcome to Maestro - Type /help to start".to_string());
    app.logs
        .push("📊 Debug mode active (Tab to focus readiness, 1-9 for direct actions)".to_string());

    let root = std::env::current_dir()?;
    let governance = MarkdownGovernance::new(root);
    let _ = governance.ensure_directories();
    let root_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Auto-bootstrap configuration if missing
    match crate::application::readiness::auto_bootstrap_config(&root_path) {
        Ok(true) => {
            app.logs
                .push("✅ Auto-configured maestro/config.yaml (Ollama detected!)".to_string());
        }
        Ok(false) => {
            // Config already exists
        }
        Err(e) => {
            app.logs
                .push(format!("⚠️ Could not auto-bootstrap config: {}", e));
        }
    }

    let loading_state = crate::application::readiness::run_checks(&root_path);
    app = app.with_readiness(loading_state);

    for check in &app.readiness.items {
        if !check.passed {
            app.logs.push(format!(
                "⚠️ [Action Required] {}: {}",
                check.name, check.dummy_guide
            ));
        }
    }

    let should_enter_interview = should_enter_interview(_bootstrap, app.readiness.is_ready());

    if should_enter_interview {
        app.logs
            .push("💬 Starting setup interview with Maestro...".to_string());
        app.mode = UIMode::Interview;
        app.interview_bot = Some(Arc::new(
            crate::application::interview_bot::InterviewBot::new(),
        ));
        let session = crate::application::interview_bot::InterviewSession::default();
        app.interview_session = Some(Arc::new(tokio::sync::RwLock::new(session)));

        if matches!(_bootstrap, OnboardingBootstrap::InitInterview) && app.readiness.is_ready() {
            app.logs
                .push("🟢 Readiness is green. Waking up Maestro persona...".to_string());
            let awake =
                run_maestro_wakeup_check(&mut app, environment.as_ref(), runtime.as_ref()).await;
            if awake {
                enqueue_interview_question(&mut app, environment.as_ref()).await?;
            }
        } else {
            enqueue_interview_question(&mut app, environment.as_ref()).await?;
        }
    } else if matches!(_bootstrap, OnboardingBootstrap::Fast) {
        app.logs.push(
            "🚀 Fast onboarding is using safe defaults. Run 'maestro onboarding --mode detailed' for guided setup."
                .to_string(),
        );
    } else if matches!(_bootstrap, OnboardingBootstrap::DirectiveGovernance) {
        app.logs.push(
            "📋 Architect Mode — directive governance. Pick a directive to author (Esc to monitor)."
                .to_string(),
        );
        app.enter_directive_select(&governance);
    }

    let mut events = EventStream::new();
    let mut ticker = tokio::time::interval(Duration::from_millis(150));

    let result = async {
        loop {
            let health = if let Some(rt) = &runtime {
                rt.health_snapshot().await
            } else {
                std::collections::HashMap::new()
            };
            app.update_agents_from_health(&health);

            if app.play_bell {
                app.play_bell = false;
                print!("\x07");
                let _ = io::stdout().flush();
            }

            let history = if let Some(env) = &environment {
                env.get_history().await
            } else {
                Vec::new()
            };
            app.update_logs_from_history(&history);

            let runtime_events = if let Some(rt) = &runtime {
                rt.events_snapshot().await
            } else {
                Vec::new()
            };
            if !runtime_events.is_empty() {
                app.update_logs_from_runtime_events(&runtime_events);
            }

            terminal.draw(|frame| render(frame, &app))?;

            tokio::select! {
                _ = ticker.tick() => {
                    app.tick_animation();
                    if app.animation_frame.is_multiple_of(20) {
                        app.refresh_readiness(&governance);
                    }
                }
                maybe_event = events.next() => {
                    if let Some(Ok(Event::Key(key))) = maybe_event {
                        match app.handle_key_event(key) {
                            Some(UserAction::Quit) => break,
                            Some(UserAction::SubmitCommand(command)) => {
                                let message = Message::new("user".to_string(), command.clone(), None);
                                app.logs.push(format!("you: {}", command));
                                let use_sequential = if let Some(rt) = &runtime {
                                    rt.has_sequential_pipeline().await
                                } else {
                                    false
                                };
                                if use_sequential {
                                    if let Some(rt) = &runtime {
                                        let rt_clone = Arc::clone(rt);
                                        tokio::spawn(async move {
                                            let _ = rt_clone
                                                .orchestrate_user_message(
                                                    message,
                                                    std::time::Duration::from_secs(5),
                                                )
                                                .await;
                                        });
                                    }
                                } else if let Some(env) = &environment {
                                    let _ = env.publish(message).await;
                                } else {
                                    app.logs.push(
                                        "⚠️ No active environment. Configure provider/model and restart Maestro."
                                            .to_string(),
                                    );
                                }
                            }
                            #[allow(clippy::collapsible_match)]
                            Some(UserAction::CompleteWizard(submission)) => {
                                if app.apply_wizard_submission(&governance, submission).is_ok() {
                                    TelemetryStore::record("wizard_completed", None)?;
                                }
                            }
                            Some(UserAction::RunReadinessAction(action)) => {
                                app.execute_readiness_action(action, &governance);
                            }
                            Some(UserAction::ProcessInterviewAnswer(answer)) => {
                                app.logs.push(format!("you: {}", answer));

                                if app.maestro_message_id.is_none() {
                                    app.logs.push(
                                        "⚠️ Maestro is not answering yet. Configure provider/model in maestro/config.yaml and restart interview."
                                            .to_string(),
                                    );
                                    continue;
                                }

                                let is_directive = if let Some(session_lock) = &app.interview_session {
                                    session_lock.read().await.target.is_some()
                                } else {
                                    false
                                };

                                if is_directive {
                                    if let Some(session_lock) = &app.interview_session {
                                        let mut session = session_lock.write().await;
                                        if let Some(exchange) = session.exchange_history.last_mut() {
                                            if exchange.user_answer.is_empty() {
                                                exchange.user_answer = answer.clone();
                                                exchange.timestamp = SystemTime::now();
                                            }
                                        }
                                        session.turn_count += 1;
                                    }
                                    let _ = enqueue_directive_question(&mut app, environment.as_ref()).await?;
                                } else {
                                    if let Some(env) = &environment {
                                        let _ = env
                                            .publish(Message::new(
                                                "user".to_string(),
                                                answer.clone(),
                                                None,
                                            ))
                                            .await;
                                    }

                                    if let (Some(bot), Some(session_lock)) =
                                        (&app.interview_bot, &app.interview_session)
                                    {
                                        let message_id = app.maestro_message_id.unwrap_or_else(Uuid::new_v4);
                                        {
                                            let mut session = session_lock.write().await;
                                            bot.process_user_answer(&mut session, answer, message_id).await?;
                                            if session.approval_pending {
                                                app.approval_modal_visible = true;
                                            }
                                        }

                                        if !app.approval_modal_visible {
                                            let _ = enqueue_interview_question(&mut app, environment.as_ref()).await?;
                                        }
                                    } else {
                                        app.logs.push(
                                            "⚠️ Interview state unavailable. Restart onboarding interview.".to_string(),
                                        );
                                    }
                                }
                            }
                            Some(UserAction::StartDirectiveAuthoring {
                                target,
                                operation,
                                file_name,
                            }) => {
                                let path = match &target {
                                    crate::application::interview_bot::DirectiveTarget::Persona { .. } => {
                                        governance.personas_dir().join(&file_name)
                                    }
                                    crate::application::interview_bot::DirectiveTarget::Skill {
                                        persona,
                                        ..
                                    } => governance.skills_dir().join(persona).join(&file_name),
                                    crate::application::interview_bot::DirectiveTarget::Scope { .. } => {
                                        governance.scopes_dir().join(&file_name)
                                    }
                                };
                                let existing = governance.read_document(&path).ok();
                                match crate::application::interview_bot::InterviewSession::for_directive(
                                    operation,
                                    target,
                                    Some(file_name),
                                    existing,
                                ) {
                                    Ok(session) => {
                                        app.interview_bot = Some(Arc::new(
                                            crate::application::interview_bot::InterviewBot::new(),
                                        ));
                                        app.interview_session =
                                            Some(Arc::new(tokio::sync::RwLock::new(session)));
                                        app.architect_picker = None;
                                        app.mode = UIMode::Interview;
                                        app.approval_modal_visible = false;
                                        let _ = enqueue_directive_question(&mut app, environment.as_ref())
                                            .await?;
                                    }
                                    Err(error) => {
                                        app.logs
                                            .push(format!("⚠️ Cannot author directive: {error}"));
                                    }
                                }
                            }
                            Some(UserAction::ManageProjectDeps) => {
                                let deps_path = root_path.join("maestro").join("project-deps.yaml");
                                if !deps_path.exists() {
                                    if let Some(parent) = deps_path.parent() {
                                        let _ = fs::create_dir_all(parent);
                                    }
                                    if fs::write(&deps_path, DEFAULT_PROJECT_DEPS_TEMPLATE).is_ok() {
                                        app.logs.push(format!(
                                            "🧩 Created project deps manifest at {}",
                                            deps_path.display()
                                        ));
                                    } else {
                                        app.logs.push(
                                            "⚠️ Failed to create maestro/project-deps.yaml from interview mode"
                                                .to_string(),
                                        );
                                        continue;
                                    }
                                } else {
                                    app.logs.push(format!(
                                        "🧩 Project deps manifest already exists at {}",
                                        deps_path.display()
                                    ));
                                }

                                app.logs.push(
                                    "✍️ Edit maestro/project-deps.yaml to add required tools and checks."
                                        .to_string(),
                                );
                                app.logs.push(
                                    "✅ Validate with: maestro deps check --scope project".to_string(),
                                );

                                if let Some(env) = &environment {
                                    let _ = env
                                        .publish(Message::new(
                                            "Maestro".to_string(),
                                            "Project deps helper: edit maestro/project-deps.yaml and run 'maestro deps check --scope project'."
                                                .to_string(),
                                            app.maestro_message_id,
                                        ))
                                        .await;
                                }
                            }
                            Some(UserAction::ApproveInterviewProposals) => {
                                app.approval_modal_visible = false;

                                let is_directive = if let Some(session_lock) = &app.interview_session {
                                    session_lock.read().await.target.is_some()
                                } else {
                                    false
                                };

                                if is_directive {
                                    match apply_directive_proposal(&mut app, &governance).await {
                                        Ok(path) => app
                                            .logs
                                            .push(format!("✅ Directive saved: {}", path.display())),
                                        Err(error) => app
                                            .logs
                                            .push(format!("❌ Failed to save directive: {error}")),
                                    }
                                    app.interview_session = None;
                                    app.interview_bot = None;
                                    app.maestro_message_id = None;
                                    app.mode = UIMode::Workspace;
                                } else {
                                    app.logs
                                        .push("✅ Proposals approved! Calling Product and applying scopes...".to_string());
                                    let applied = apply_interview_scope_proposals(
                                        &mut app,
                                        &governance,
                                        environment.as_ref(),
                                    )
                                    .await?;
                                    app.logs.push(format!(
                                        "✅ Product scope handoff completed. {} scope draft(s) applied.",
                                        applied
                                    ));
                                    app.mode = UIMode::Workspace;
                                }
                            }
                            Some(UserAction::RejectInterviewProposals) => {
                                app.approval_modal_visible = false;
                                if let Some(session_lock) = &app.interview_session {
                                    let mut session = session_lock.write().await;
                                    session.approval_pending = false;
                                    session.proposed_changes = None;
                                }
                                app.logs
                                    .push("❓ Understood. Let us refine requirements before generating new scope drafts.".to_string());
                                let _ = enqueue_interview_question(&mut app, environment.as_ref()).await?;
                            }
                            Some(UserAction::Logout) => {
                                let _ = GeminiAdapter::clear_credentials();
                                app.logs.push("🎼 ✅ Logout completed successfully (OS Keyring cleared).".to_string());
                            }
                            None => {}
                        }
                    }
                }
            }
        }

        Ok::<(), anyhow::Error>(())
    }
    .await;

    restore_terminal(terminal)?;
    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(EnableBlinking)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(mut terminal: Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.backend_mut().execute(DisableBlinking)?;
    terminal.show_cursor()?;
    Ok(())
}

fn map_health(health: &AgentHealth) -> &'static str {
    match health {
        AgentHealth::Starting | AgentHealth::Idle | AgentHealth::Stopped => "idle",
        AgentHealth::Observing => "observe",
        AgentHealth::Thinking => "think",
        AgentHealth::Acting => "act",
        AgentHealth::Failed => "error",
    }
}

fn status_color(status: &str) -> Color {
    match status {
        "observe" => Color::Yellow,
        "think" => Color::Cyan,
        "act" => Color::Green,
        "error" => Color::Red,
        _ => Color::White,
    }
}

#[cfg(test)]
mod tests;

mod architect;
use architect::*;

mod telemetry;
use telemetry::*;

mod interview;
use interview::*;

mod wizard;
pub use wizard::WizardSubmission;
use wizard::*;

mod render;
use render::*;

mod app;
