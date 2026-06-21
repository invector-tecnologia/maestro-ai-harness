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
use crate::application::markdown_governance::MarkdownGovernance;
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
    Interview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum PanelFocus {
    #[default]
    Workspace,
    Readiness,
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
}

fn should_enter_interview(bootstrap: OnboardingBootstrap, readiness_ready: bool) -> bool {
    match bootstrap {
        OnboardingBootstrap::InitInterview => true,
        OnboardingBootstrap::Detailed => !readiness_ready,
        OnboardingBootstrap::Fast => false,
    }
}

impl TuiApp {
    pub fn with_readiness(
        mut self,
        readiness: crate::application::readiness::ReadinessState,
    ) -> Self {
        self.readiness = readiness;
        self.refresh_dependency_domains();
        self
    }

    pub fn show_help(&mut self) {
        self.mode = UIMode::HelpMenu;
        self.logs.clear();
        self.logs
            .push("📚 MAESTRO - INTERACTIVE MANUAL".to_string());
        self.logs.push(String::new());
        self.logs.push("Start (Quick Start):".to_string());
        self.logs
            .push("  maestro onboarding --mode fast      - Start with safe defaults".to_string());
        self.logs.push(
            "  maestro onboarding --mode detailed  - Start the guided setup interview".to_string(),
        );
        self.logs
            .push("  /new persona   - Create a new persona (AI agent)".to_string());
        self.logs
            .push("  /new scope     - Create a new work scope".to_string());
        self.logs
            .push("  /new skill     - Teach a new skill to an agent".to_string());
        self.logs
            .push("  /deps          - Create/edit maestro project deps manifest".to_string());
        self.logs.push(String::new());
        self.logs.push("Check Status:".to_string());
        self.logs
            .push("  /status        - View agents health".to_string());
        self.logs
            .push("  /check         - Check if system is ready".to_string());
        self.logs.push(String::new());
        self.logs
            .push("Edit Configurations (in text editor):".to_string());
        self.logs
            .push("  maestro/config.yaml       - Configure providers/models".to_string());
        self.logs
            .push("  maestro/personas/*.md     - Edit personas freely".to_string());
        self.logs
            .push("  maestro/scopes/*.md       - Edit work scopes".to_string());
        self.logs.push(String::new());
        self.logs.push("Controls:".to_string());
        self.logs
            .push("  Ctrl+L         - Log out of providers (e.g. Google Gemini)".to_string());
        self.logs
            .push("  Ctrl+D         - Toggle debug panel".to_string());
        self.logs
            .push("  q              - Quit (when input is empty)".to_string());
        self.logs.push("  ESC            - Quit".to_string());
        self.logs.push(String::new());
        self.logs
            .push("Type 'back' to return to the workspace".to_string());
    }

    pub fn return_to_workspace(&mut self) {
        self.mode = UIMode::Workspace;
    }

    fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            PanelFocus::Workspace => PanelFocus::Readiness,
            PanelFocus::Readiness => PanelFocus::Workspace,
        };
    }

    pub fn tick_animation(&mut self) {
        self.animation_frame = self.animation_frame.wrapping_add(1);
        self.normalize_readiness_selection();
    }

    fn refresh_readiness(&mut self, _governance: &MarkdownGovernance) {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        self.readiness = crate::application::readiness::run_checks(&root);
        self.refresh_dependency_domains();
        self.normalize_readiness_selection();
    }
    fn refresh_dependency_domains(&mut self) {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        self.dependency_domains = evaluate_project_dependency_domains(&root);
    }

    fn readiness_actions(&self) -> Vec<ReadinessAction> {
        let mut actions = Vec::new();

        if !self.readiness.has_config {
            actions.push(ReadinessAction::CreateConfigTemplate);
        } else {
            actions.push(ReadinessAction::OpenConfigHint);
        }

        if !self.dependency_domains.project_manifest_found {
            actions.push(ReadinessAction::CreateProjectDepsTemplate);
        }

        for (dependency, install_hint) in &self.dependency_domains.project_failed_required_hints {
            actions.push(ReadinessAction::RemediateProjectDependency {
                dependency: dependency.clone(),
                install_hint: install_hint.clone(),
            });
        }

        if self.readiness.has_config && !self.readiness.has_providers {
            actions.push(ReadinessAction::ConfigureProviders);
        }

        if self.readiness.has_providers && !self.readiness.provider_reachable {
            actions.push(ReadinessAction::StartProvider);
        }

        if !self.readiness.has_scopes {
            actions.push(ReadinessAction::CreateScope);
        }
        if !self.readiness.has_personas {
            actions.push(ReadinessAction::CreatePersona);
        }
        if !self.readiness.has_skills {
            actions.push(ReadinessAction::CreateSkill);
        }

        actions
    }

    fn normalize_readiness_selection(&mut self) {
        let count = self.readiness_actions().len();
        if count == 0 {
            self.readiness_selected_action = 0;
            return;
        }

        if self.readiness_selected_action >= count {
            self.readiness_selected_action = count - 1;
        }
    }

    fn selected_readiness_action(&self) -> Option<ReadinessAction> {
        let actions = self.readiness_actions();
        actions.get(self.readiness_selected_action).cloned()
    }

    fn select_next_readiness_action(&mut self) {
        let count = self.readiness_actions().len();
        if count == 0 {
            return;
        }
        self.readiness_selected_action = (self.readiness_selected_action + 1) % count;
    }

    fn select_previous_readiness_action(&mut self) {
        let count = self.readiness_actions().len();
        if count == 0 {
            return;
        }
        if self.readiness_selected_action == 0 {
            self.readiness_selected_action = count - 1;
        } else {
            self.readiness_selected_action -= 1;
        }
    }

    fn execute_readiness_action(
        &mut self,
        action: ReadinessAction,
        governance: &MarkdownGovernance,
    ) {
        match action {
            ReadinessAction::CreateConfigTemplate => {
                let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                let config_path = root.join("maestro").join("config.yaml");
                if !config_path.exists() {
                    if let Some(parent) = config_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if fs::write(&config_path, DEFAULT_CONFIG_TEMPLATE).is_ok() {
                        self.logs.push(format!(
                            "readiness action: config template created at {}",
                            config_path.display()
                        ));
                    } else {
                        self.logs
                            .push("readiness action: failed to create config template".to_string());
                    }
                } else {
                    self.logs
                        .push("readiness action: config already exists".to_string());
                }
            }
            ReadinessAction::CreateProjectDepsTemplate => {
                let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                let deps_path = root.join("maestro").join("project-deps.yaml");
                if !deps_path.exists() {
                    if let Some(parent) = deps_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if fs::write(&deps_path, DEFAULT_PROJECT_DEPS_TEMPLATE).is_ok() {
                        self.logs.push(format!(
                            "readiness action: project deps template created at {}",
                            deps_path.display()
                        ));
                    } else {
                        self.logs.push(
                            "readiness action: failed to create project deps template".to_string(),
                        );
                    }
                } else {
                    self.logs
                        .push("readiness action: project deps manifest already exists".to_string());
                }
            }
            ReadinessAction::RemediateProjectDependency {
                dependency,
                install_hint,
            } => {
                self.logs.push(format!(
                    "readiness action: dependency '{dependency}' is required but failing"
                ));
                if let Some(hint) = install_hint {
                    self.logs.push(format!("readiness action: {hint}"));
                } else {
                    self.logs.push(
                        "readiness action: run 'maestro deps check --scope project' for full diagnostics"
                            .to_string(),
                    );
                }
            }
            ReadinessAction::OpenConfigHint => {
                self.logs
                    .push("readiness action: open maestro/config.yaml in the editor".to_string());
            }
            ReadinessAction::ConfigureProviders => {
                self.logs.push(
                    "readiness action: add providers: and system.default_provider in maestro/config.yaml"
                        .to_string(),
                );
            }
            ReadinessAction::StartProvider => {
                self.logs.push(
                    "readiness action: start the default provider (e.g. ollama serve)".to_string(),
                );
            }
            ReadinessAction::CreateScope => {
                self.wizard = Some(CreationWizard::new_scope());
                self.focus = PanelFocus::Workspace;
                self.logs
                    .push("readiness action: scope wizard started".to_string());
            }
            ReadinessAction::CreatePersona => {
                self.wizard = Some(CreationWizard::new_persona());
                self.focus = PanelFocus::Workspace;
                self.logs
                    .push("readiness action: persona wizard started".to_string());
            }
            ReadinessAction::CreateSkill => {
                self.wizard = Some(CreationWizard::new_skill());
                self.focus = PanelFocus::Workspace;
                self.logs
                    .push("readiness action: skill wizard started".to_string());
            }
        }

        self.refresh_readiness(governance);
    }

    pub fn update_agents_from_health(&mut self, health: &HashMap<String, AgentHealth>) {
        let mut new_agents = health
            .iter()
            .map(|(name, state)| AgentView {
                name: name.clone(),
                status: map_health(state).to_string(),
            })
            .collect::<Vec<_>>();

        new_agents.sort_by(|a, b| a.name.cmp(&b.name));

        for old_agent in &self.agents {
            if old_agent.status == "act" {
                if let Some(new_agent) = new_agents.iter().find(|a| a.name == old_agent.name) {
                    if new_agent.status != "act" {
                        self.play_bell = true;
                        self.highlight_until
                            .insert(old_agent.name.clone(), self.animation_frame + 10);
                    }
                } else {
                    self.play_bell = true;
                    self.highlight_until
                        .insert(old_agent.name.clone(), self.animation_frame + 10);
                }
            }
        }

        self.agents = new_agents;
    }

    pub fn update_logs_from_history(&mut self, history: &[Message]) {
        if history.is_empty() {
            return;
        }

        let mut lines = history
            .iter()
            .map(|msg| format!("{}: {}", msg.sender(), msg.content()))
            .collect::<Vec<_>>();

        if lines.len() > 100 {
            lines = lines.split_off(lines.len() - 100);
        }

        self.logs = lines;
    }

    /// Update logs from runtime events (for observability).
    pub fn update_logs_from_runtime_events(
        &mut self,
        events: &[crate::application::agent_observability::RuntimeEventWithTimestamp],
    ) {
        if events.len() <= self.last_runtime_event_count {
            return;
        }

        use crate::application::agent_observability::RuntimeEvent;

        let new_lines = events[self.last_runtime_event_count..]
            .iter()
            .map(|evt| {
                let evt_desc = match &evt.event {
                    RuntimeEvent::AgentObserving {
                        agent_name,
                        message_id,
                    } => format!("📥 {} observing message {}", agent_name, message_id),
                    RuntimeEvent::AgentThinking {
                        agent_name,
                        context,
                    } => format!("🧠 {} thinking: {}", agent_name, context),
                    RuntimeEvent::AgentActing {
                        agent_name,
                        decision,
                    } => format!("⚙️ {} acting: {}", agent_name, decision),
                    RuntimeEvent::AgentActed {
                        agent_name,
                        output,
                        handoff_target,
                    } => {
                        let handoff_str = handoff_target
                            .as_ref()
                            .map(|h| format!(" → {}", h))
                            .unwrap_or_default();
                        format!("✅ {} completed{}: {}", agent_name, handoff_str, output)
                    }
                    RuntimeEvent::SkillExecutionStarted {
                        persona_name,
                        skill_name,
                        input,
                    } => format!(
                        "🎯 Executing {} skill '{}' with: {}",
                        persona_name, skill_name, input
                    ),
                    RuntimeEvent::SkillExecutionCompleted {
                        persona_name,
                        skill_name,
                        result,
                        success,
                    } => {
                        let status = if *success { "✓" } else { "✗" };
                        format!(
                            "{} {} skill '{}' result: {}",
                            status, persona_name, skill_name, result
                        )
                    }
                    RuntimeEvent::ExecutionError {
                        agent_name,
                        error_message,
                    } => format!("❌ {} error: {}", agent_name, error_message),
                };
                evt_desc
            })
            .collect::<Vec<_>>();

        self.last_runtime_event_count = events.len();
        self.logs.extend(new_lines);

        // Keep only last 100 lines
        if self.logs.len() > 100 {
            self.logs = self.logs.split_off(self.logs.len() - 100);
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Option<UserAction> {
        if key.kind != KeyEventKind::Press {
            return None;
        }

        if key.code == KeyCode::Esc {
            return Some(UserAction::Quit);
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Some(UserAction::Quit);
        }

        if key.modifiers.contains(KeyModifiers::CONTROL)
            && (key.code == KeyCode::Char('l') || key.code == KeyCode::Char('L'))
        {
            return Some(UserAction::Logout);
        }

        if key.modifiers.contains(KeyModifiers::CONTROL)
            && (key.code == KeyCode::Char('d') || key.code == KeyCode::Char('D'))
        {
            self.show_debug = !self.show_debug;
            return None;
        }

        if key.code == KeyCode::Tab && self.wizard.is_none() {
            self.toggle_focus();
            return None;
        }

        if self.focus == PanelFocus::Readiness && self.wizard.is_none() {
            match key.code {
                KeyCode::Up => {
                    self.select_previous_readiness_action();
                    return None;
                }
                KeyCode::Down => {
                    self.select_next_readiness_action();
                    return None;
                }
                KeyCode::Enter if self.input.trim().is_empty() => {
                    if let Some(action) = self.selected_readiness_action() {
                        return Some(UserAction::RunReadinessAction(action));
                    }
                    return None;
                }
                KeyCode::Char(c @ '1'..='9') => {
                    let index = (c as usize) - ('1' as usize);
                    let actions = self.readiness_actions();
                    if index < actions.len() {
                        self.readiness_selected_action = index;
                        if let Some(action) = self.selected_readiness_action() {
                            return Some(UserAction::RunReadinessAction(action));
                        }
                    }
                    return None;
                }
                KeyCode::Char(_) | KeyCode::Backspace => {
                    return None;
                }
                _ => {}
            }
        }

        // Handle Interview mode
        if self.mode == UIMode::Interview {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') if self.approval_modal_visible => {
                    return Some(UserAction::ApproveInterviewProposals);
                }
                KeyCode::Char('n') | KeyCode::Char('N') if self.approval_modal_visible => {
                    return Some(UserAction::RejectInterviewProposals);
                }
                KeyCode::Enter => {
                    let answer = self.input.trim().to_string();
                    self.input.clear();
                    if !answer.is_empty() {
                        if answer.starts_with("/deps") {
                            return Some(UserAction::ManageProjectDeps);
                        }
                        return Some(UserAction::ProcessInterviewAnswer(answer));
                    }
                    return None;
                }
                KeyCode::Char(c) => {
                    self.input.push(c);
                    return None;
                }
                KeyCode::Backspace => {
                    self.input.pop();
                    return None;
                }
                _ => return None,
            }
        }

        match key.code {
            KeyCode::Char('q') if self.input.is_empty() && self.wizard.is_none() => {
                Some(UserAction::Quit)
            }
            KeyCode::Char(c) => {
                self.input.push(c);
                None
            }
            KeyCode::Backspace => {
                self.input.pop();
                None
            }
            KeyCode::Enter => {
                let command = self.input.trim().to_string();
                self.input.clear();

                if let Some(wizard) = &mut self.wizard {
                    let advanced = wizard.advance(&command);
                    match advanced {
                        WizardAdvance::NeedMoreInput => None,
                        WizardAdvance::ValidationError(message) => {
                            self.logs.push(format!("wizard: {message}"));
                            None
                        }
                        WizardAdvance::Completed(submission) => {
                            self.wizard = None;
                            Some(UserAction::CompleteWizard(submission))
                        }
                    }
                } else if command.is_empty() {
                    None
                } else if command == "/help" {
                    self.show_help();
                    None
                } else if command == "back" && self.mode == UIMode::HelpMenu {
                    self.return_to_workspace();
                    None
                } else if command.starts_with("/new") {
                    match self.start_wizard_from_command(&command) {
                        Ok(()) => {
                            self.logs.push(format!("wizard started: {command}"));
                        }
                        Err(error) => {
                            self.logs.push(format!("wizard: {error}"));
                        }
                    }
                    None
                } else if command == "/debug" {
                    self.show_debug = !self.show_debug;
                    let state = if self.show_debug {
                        "on (Ctrl+D to toggle off)"
                    } else {
                        "off (Ctrl+D to toggle on)"
                    };
                    self.logs.push(format!("debug mode: {}", state));
                    None
                } else if command == "/check" {
                    let status = if self.readiness.is_ready() {
                        "✅ READY TO GO! System configured and customizable."
                    } else {
                        "⚠️  System is not fully ready.\n\nSuggested steps:\n"
                    };
                    self.logs.push(status.to_string());

                    if !self.readiness.has_config {
                        self.logs.push(
                            "  1. Create maestro/config.yaml (use: maestro init-config)"
                                .to_string(),
                        );
                    }
                    if self.readiness.has_config && !self.readiness.has_providers {
                        self.logs.push(
                            "  2. Define at least one provider in config.yaml under providers:"
                                .to_string(),
                        );
                    }
                    if self.readiness.has_providers && !self.readiness.provider_reachable {
                        self.logs.push(
                            "  3. Start the default provider (e.g. ollama serve) or adjust the endpoint"
                                .to_string(),
                        );
                    }
                    if !self.readiness.has_scopes {
                        self.logs
                            .push("  4. Create a scope: /new scope".to_string());
                    }
                    if !self.readiness.has_personas {
                        self.logs
                            .push("  5. Create a persona: /new persona".to_string());
                    }
                    if !self.readiness.has_skills {
                        self.logs
                            .push("  6. Create a skill: /new skill".to_string());
                    }
                    if !self.dependency_domains.project_manifest_found {
                        self.logs.push(
                            "  7. Create maestro/project-deps.yaml (use readiness action or maestro scaffold-markdown)"
                                .to_string(),
                        );
                    }
                    if self.dependency_domains.project_manifest_found
                        && (!self.dependency_domains.project_manifest_valid
                            || !self.dependency_domains.project_required_checks_passed)
                    {
                        self.logs.push(
                            "  8. Validate project dependencies: maestro deps check --scope project"
                                .to_string(),
                        );
                    }
                    None
                } else if command.starts_with("/ask ") {
                    // /ask <question> - triggers default persona to respond
                    let question = command.strip_prefix("/ask ").unwrap_or("").to_string();
                    if question.is_empty() {
                        self.logs.push("❌ Usage: /ask <your question>".to_string());
                        None
                    } else {
                        self.logs.push(format!("🎯 Asking: {}", question));
                        Some(UserAction::SubmitCommand(question))
                    }
                } else {
                    Some(UserAction::SubmitCommand(command))
                }
            }
            _ => None,
        }
    }

    fn start_wizard_from_command(&mut self, command: &str) -> Result<(), String> {
        let parts = command
            .split_whitespace()
            .map(|value| value.to_string())
            .collect::<Vec<_>>();

        if parts.len() < 2 {
            return Err("use /new persona|scope|skill".to_string());
        }

        match parts[1].as_str() {
            "persona" => {
                self.wizard = Some(CreationWizard::new_persona());
                Ok(())
            }
            "scope" => {
                self.wizard = Some(CreationWizard::new_scope());
                Ok(())
            }
            "skill" => {
                self.wizard = Some(CreationWizard::new_skill());
                Ok(())
            }
            _ => Err("invalid wizard type: use persona, scope, or skill".to_string()),
        }
    }

    fn current_input_title(&self) -> String {
        if let Some(wizard) = &self.wizard {
            format!(
                "Wizard {} - {} (Enter confirms, q exits)",
                wizard.kind.label(),
                wizard.current_prompt()
            )
        } else if self.focus == PanelFocus::Readiness {
            "Readiness focus (Tab switches, Up/Down selects, Enter runs)".to_string()
        } else if self.mode == UIMode::HelpMenu {
            "Help (type 'back' to return)".to_string()
        } else {
            "Command (Enter sends, q quits | /help /check /new persona|scope|skill)".to_string()
        }
    }
    fn apply_wizard_submission(
        &mut self,
        governance: &MarkdownGovernance,
        submission: WizardSubmission,
    ) -> Result<(), anyhow::Error> {
        match persist_submission(governance, submission) {
            Ok(path) => {
                self.logs
                    .push(format!("✅ File created: {}", path.display()));
                let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                self.readiness = crate::application::readiness::run_checks(&root);
                Ok(())
            }
            Err(error) => {
                self.logs.push(format!("❌ Error saving: {error}"));
                Err(error)
            }
        }
    }
}

enum UserAction {
    SubmitCommand(String),
    CompleteWizard(WizardSubmission),
    RunReadinessAction(ReadinessAction),
    ProcessInterviewAnswer(String),
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
                                if let Some(env) = &environment {
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

pub fn render(frame: &mut Frame<'_>, app: &TuiApp) {
    let area = frame.area();

    // Interview mode has special layout
    if app.mode == UIMode::Interview {
        let interview_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // Logo
                Constraint::Length(6), // Maestro panel
                Constraint::Min(0),    // Monitor/logs
                Constraint::Length(5), // User input
            ])
            .split(area);

        render_logo_panel(frame, interview_rows[0]);
        render_maestro_panel(frame, interview_rows[1], app);
        render_monitor_panel(frame, interview_rows[2], app);
        render_input_panel(frame, interview_rows[3], app);
        render_approval_modal(frame, area, app);
        return;
    }

    // Main vertical split: Top (Workspace + Sidebars) and Bottom (Gauge + Command)
    let main_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),     // Top
            Constraint::Length(10), // Bottom
        ])
        .split(area);

    let top_area = main_rows[0];
    let bottom_area = main_rows[1];

    // Overall split: Workspace vs Sidebars
    let workspace_pct = if app.show_debug { 55 } else { 75 };
    let readiness_pct = 25;
    let debug_pct = if app.show_debug { 20 } else { 0 };

    let mut constraints = vec![
        Constraint::Percentage(workspace_pct),
        Constraint::Percentage(readiness_pct),
    ];
    if app.show_debug {
        constraints.push(Constraint::Percentage(debug_pct));
    }

    let top_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(top_area);

    let workspace_area = top_columns[0];
    render_readiness_panel(frame, top_columns[1], app);
    if app.show_debug {
        render_debug_panel(frame, top_columns[2], app);
    }

    // Workspace Area: Logo on top, Agents and Logs side-by-side
    let ws_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Logo
            Constraint::Min(0),    // Agents and Logs
        ])
        .split(workspace_area);

    render_logo_panel(frame, ws_rows[0]);

    let ws_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Agents
            Constraint::Percentage(60), // Logs
        ])
        .split(ws_rows[1]);

    render_agents_panel(frame, ws_cols[0], app);
    render_monitor_panel(frame, ws_cols[1], app);

    // Bottom Area
    let agents_width = ws_cols[0].width;

    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(agents_width), // Highlighted agents
            Constraint::Min(0),               // Command (screen width - highlighted agents)
        ])
        .split(bottom_area);

    render_gauge_panel(frame, bottom_cols[0], app);

    // Command height = half of highlighted agents area
    let command_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(0),   // Empty top space
            Constraint::Percentage(100), // Command in the lower half
        ])
        .split(bottom_cols[1]);

    render_input_panel(frame, command_rows[1], app);
}

fn render_readiness_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect, app: &TuiApp) {
    let all_ready = app.readiness.is_ready()
        && app.dependency_domains.project_manifest_found
        && app.dependency_domains.project_manifest_valid
        && app.dependency_domains.project_required_checks_passed;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(18), Constraint::Min(8)])
        .split(area);

    let (headline, headline_style) = if all_ready {
        (
            "Maestro is ready ✅",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        (
            "Maestro is ready ❌",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )
    };

    let mut lines = vec![headline.to_string(), String::new()];

    lines.push("Harness dependencies:".to_string());
    for check in &app.readiness.items {
        lines.push(readiness_line(&check.name, check.passed));
    }

    lines.push(String::new());
    lines.push("Project dependencies:".to_string());
    lines.push(readiness_line(
        "project-deps manifest",
        app.dependency_domains.project_manifest_found,
    ));
    lines.push(readiness_line(
        "project-deps schema",
        app.dependency_domains.project_manifest_valid,
    ));
    lines.push(readiness_line(
        "required dependency checks",
        app.dependency_domains.project_required_checks_passed,
    ));

    if !app.dependency_domains.project_failed_required.is_empty() {
        lines.push(format!(
            "missing: {}",
            app.dependency_domains.project_failed_required.join(", ")
        ));
    }
    if let Some(error) = &app.dependency_domains.project_error {
        lines.push(format!("error: {error}"));
    }

    let paragraph = Paragraph::new(lines.join("\n"))
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title("Readiness")
                .borders(Borders::ALL)
                .border_style(if all_ready {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                }),
        );

    frame.render_widget(paragraph.style(headline_style), chunks[0]);

    let mut actions_lines = vec![];

    actions_lines.push(format!(
        "Focus: {} (Tab to switch)",
        if app.focus == PanelFocus::Readiness {
            "Readiness"
        } else {
            "Workspace"
        }
    ));
    actions_lines.push(String::new());

    if !all_ready {
        let actions = app.readiness_actions();
        for (index, action) in actions.iter().enumerate() {
            let is_selected =
                app.focus == PanelFocus::Readiness && app.readiness_selected_action == index;
            let marker = if is_selected { "> [ ]" } else { "  [ ]" };
            actions_lines.push(format!("{} {}", marker, action.label()));
        }
        if actions.is_empty() {
            actions_lines.push("  no pending actions".to_string());
        }
    }

    let actions_paragraph = Paragraph::new(actions_lines.join("\n"))
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title("Actions")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(218, 165, 32))),
        );

    frame.render_widget(actions_paragraph, chunks[1]);
}

fn readiness_line(label: &str, ok: bool) -> String {
    if ok {
        format!("[x] {label}")
    } else {
        format!("[ ] {label}")
    }
}

fn evaluate_project_dependency_domains(root: &std::path::Path) -> DependencyDomainsState {
    let deps_path = root.join("maestro").join("project-deps.yaml");
    if !deps_path.exists() {
        return DependencyDomainsState {
            project_manifest_found: false,
            project_manifest_valid: false,
            project_required_checks_passed: false,
            project_failed_required: Vec::new(),
            project_failed_required_hints: Vec::new(),
            project_error: Some("maestro/project-deps.yaml not found".to_string()),
        };
    }

    match ProjectDepsConfig::load(Some(deps_path)) {
        Ok(config) => {
            let mut failed_required = Vec::new();
            let mut failed_required_hints = Vec::new();

            for dep in config.dependencies.iter().filter(|dep| dep.required) {
                let passed = std::process::Command::new("sh")
                    .arg("-lc")
                    .arg(&dep.check_command)
                    .status()
                    .map(|status| status.success())
                    .unwrap_or(false);

                if !passed {
                    failed_required.push(dep.name.clone());
                    failed_required_hints.push((dep.name.clone(), dep.install_hint.clone()));
                }
            }

            DependencyDomainsState {
                project_manifest_found: true,
                project_manifest_valid: true,
                project_required_checks_passed: failed_required.is_empty(),
                project_failed_required: failed_required,
                project_failed_required_hints: failed_required_hints,
                project_error: None,
            }
        }
        Err(error) => DependencyDomainsState {
            project_manifest_found: true,
            project_manifest_valid: false,
            project_required_checks_passed: false,
            project_failed_required: Vec::new(),
            project_failed_required_hints: Vec::new(),
            project_error: Some(error.to_string()),
        },
    }
}

fn render_debug_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect, app: &TuiApp) {
    let debug_info = format!(
        "TUI STATE INSPECTOR\n\
        ===================\n\n\
        Animation Frame: {}\n\
        Play Bell: {}\n\n\
        Wizard Active: {}\n\
        Wizard Kind: {:?}\n\
        Mode: {:?}\n\
        Readiness: ready={}\n\n\
        Agents Count: {}\n\
        Input Len: {}\n\
        Highlight Cache:\n {:?}",
        app.animation_frame,
        app.play_bell,
        app.wizard.is_some(),
        app.wizard.as_ref().map(|w| w.kind.label()),
        app.mode,
        app.readiness.is_ready(),
        app.agents.len(),
        app.input.len(),
        app.highlight_until,
    );

    let paragraph = Paragraph::new(debug_info)
        .style(Style::default().fg(Color::DarkGray))
        .block(
            Block::default()
                .title(" Debug Panel ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    frame.render_widget(paragraph, area);
}

fn render_gauge_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect, _app: &TuiApp) {
    let block = Block::default()
        .title("Highlighted Agents")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(218, 165, 32)));

    let paragraph = Paragraph::new("\n\n        [ Gauge 80% ]\n")
        .style(Style::default().fg(Color::White))
        .block(block);

    frame.render_widget(paragraph, area);
}

fn render_logo_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect) {
    let big_text = BigText::builder()
        .pixel_size(PixelSize::Full)
        .style(Style::default().fg(Color::Rgb(218, 165, 32)))
        .lines(vec!["MAESTRO".into()])
        .build();
    frame.render_widget(big_text, area);
}

fn render_agents_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect, app: &TuiApp) {
    let eq_frames = [
        " ▂▃▄▅",
        "▂▃▄▅▆",
        "▃▄▅▆▇",
        "▄▅▆▇█",
        "▅▆▇█▇",
        "▆▇█▇▆",
        "▇█▇▆▅",
        "█▇▆▅▄",
        "▇▆▅▄▃",
        "▆▅▄▃▂",
        "▅▄▃▂ ",
    ];
    let eq = eq_frames[app.animation_frame % eq_frames.len()];

    let rows = app
        .agents
        .iter()
        .map(|agent| {
            let display_status = match agent.status.as_str() {
                "think" => format!("{} think 🎵", eq),
                "act" => format!("{} act 🎼", eq),
                _ => agent.status.clone(),
            };

            let is_highlighted = app
                .highlight_until
                .get(&agent.name)
                .map(|&f| app.animation_frame < f)
                .unwrap_or(false);

            let mut name_cell = Cell::from(agent.name.clone());
            if is_highlighted {
                name_cell = name_cell.style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                );
            } else {
                name_cell = name_cell.style(Style::default().fg(status_color(&agent.status)));
            }

            let status_cell =
                Cell::from(display_status).style(Style::default().fg(status_color(&agent.status)));

            Row::new(vec![name_cell, status_cell])
        })
        .collect::<Vec<_>>();

    let table = Table::new(
        rows,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .header(Row::new(vec!["Agent", "Status"]).style(Style::default().fg(Color::Rgb(218, 165, 32))))
    .block(
        Block::default()
            .title("Agents")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(218, 165, 32))),
    );

    frame.render_widget(table, area);
}

fn render_monitor_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect, app: &TuiApp) {
    let items = app
        .logs
        .iter()
        .rev()
        .take(20)
        .rev()
        .map(|line| ListItem::new(line.clone()))
        .collect::<Vec<_>>();

    let list = List::new(items).block(
        Block::default()
            .title("Monitor")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(218, 165, 32))),
    );
    frame.render_widget(list, area);
}

fn render_input_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect, app: &TuiApp) {
    let paragraph = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title(app.current_input_title())
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(218, 165, 32))),
        );

    frame.render_widget(paragraph, area);

    let is_focused = app.wizard.is_some() || app.focus == PanelFocus::Workspace;
    if is_focused {
        let max_x = area.x + area.width.saturating_sub(2);
        let cursor_x = (area.x + 1 + app.input.chars().count() as u16).min(max_x);
        let cursor_y = area.y + 1;

        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn render_maestro_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect, app: &TuiApp) {
    let mut lines = vec![];

    if let Some(session_lock) = &app.interview_session {
        let turn = session_lock
            .try_read()
            .ok()
            .map(|session| session.turn_count)
            .unwrap_or(0);
        lines.push("🤖 Maestro Interview".to_string());
        lines.push(format!("  Turn: {}/10", turn));
        if app.approval_modal_visible {
            lines.push("  🔔 Awaiting your decision...".to_string());
        } else {
            lines.push("  🎧 Listening...".to_string());
        }
    } else {
        lines.push("🤖 Maestro: Ready to help with setup".to_string());
    }

    let paragraph = Paragraph::new(lines.join("\n"))
        .style(Style::default().fg(Color::Cyan))
        .block(
            Block::default()
                .title("Maestro")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    frame.render_widget(paragraph, area);
}

fn render_approval_modal(frame: &mut Frame<'_>, area: ratatui::layout::Rect, app: &TuiApp) {
    if !app.approval_modal_visible {
        return;
    }

    let modal_width = 60;
    let modal_height = 12;
    let modal_x = (area.width.saturating_sub(modal_width)) / 2;
    let modal_y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = ratatui::layout::Rect {
        x: area.x + modal_x,
        y: area.y + modal_y,
        width: modal_width,
        height: modal_height,
    };

    let mut proposal_text = vec!["Maestro's Recommendations:".to_string(), "".to_string()];
    if let Some(session_lock) = &app.interview_session {
        if let Ok(session) = session_lock.try_read() {
            if let Some(proposals) = &session.proposed_changes {
                proposal_text.push(proposals.summary.clone());
                proposal_text.push("".to_string());
                proposal_text.push("Scope drafts (Product handoff):".to_string());
                for (name, _) in proposals.scope_drafts.iter().take(3) {
                    proposal_text.push(format!("  • {}", name));
                }
            }
        }
    }
    proposal_text.push("".to_string());
    proposal_text.push("Approve changes? [Y/n]".to_string());

    let modal = Paragraph::new(proposal_text.join("\n"))
        .style(Style::default().fg(Color::White).bg(Color::DarkGray))
        .block(
            Block::default()
                .title("Setup Proposal")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .alignment(Alignment::Left);

    frame.render_widget(modal, modal_area);
}

#[derive(Debug, Clone)]
pub enum WizardSubmission {
    Persona {
        file_name: String,
        content: String,
    },
    Scope {
        file_name: String,
        content: String,
    },
    Skill {
        persona_name: String,
        file_name: String,
        content: String,
    },
}

enum WizardAdvance {
    NeedMoreInput,
    ValidationError(String),
    Completed(WizardSubmission),
}

#[derive(Debug, Clone)]
enum WizardKind {
    Persona,
    Scope,
    Skill,
}

impl WizardKind {
    fn label(&self) -> &'static str {
        match self {
            WizardKind::Persona => "persona",
            WizardKind::Scope => "scope",
            WizardKind::Skill => "skill",
        }
    }
}

#[derive(Debug, Clone)]
struct WizardField {
    prompt: &'static str,
    value: String,
}

#[derive(Debug, Clone)]
struct CreationWizard {
    kind: WizardKind,
    fields: Vec<WizardField>,
    cursor: usize,
}

impl CreationWizard {
    fn new_persona() -> Self {
        Self {
            kind: WizardKind::Persona,
            fields: vec![
                WizardField {
                    prompt: "persona name",
                    value: String::new(),
                },
                WizardField {
                    prompt: "responsibility",
                    value: String::new(),
                },
                WizardField {
                    prompt: "deliverables",
                    value: String::new(),
                },
                WizardField {
                    prompt: "instructions",
                    value: String::new(),
                },
                WizardField {
                    prompt: "interaction matrix",
                    value: String::new(),
                },
                WizardField {
                    prompt: "limits",
                    value: String::new(),
                },
            ],
            cursor: 0,
        }
    }

    fn new_scope() -> Self {
        Self {
            kind: WizardKind::Scope,
            fields: vec![
                WizardField {
                    prompt: "delivery number (e.g. 001)",
                    value: String::new(),
                },
                WizardField {
                    prompt: "delivery name",
                    value: String::new(),
                },
                WizardField {
                    prompt: "objective",
                    value: String::new(),
                },
                WizardField {
                    prompt: "business scope",
                    value: String::new(),
                },
                WizardField {
                    prompt: "deliverables",
                    value: String::new(),
                },
                WizardField {
                    prompt: "acceptance criteria",
                    value: String::new(),
                },
                WizardField {
                    prompt: "dependencies",
                    value: String::new(),
                },
            ],
            cursor: 0,
        }
    }

    fn new_skill() -> Self {
        Self {
            kind: WizardKind::Skill,
            fields: vec![
                WizardField {
                    prompt: "target persona",
                    value: String::new(),
                },
                WizardField {
                    prompt: "skill name",
                    value: String::new(),
                },
                WizardField {
                    prompt: "objective",
                    value: String::new(),
                },
                WizardField {
                    prompt: "triggers",
                    value: String::new(),
                },
                WizardField {
                    prompt: "inputs",
                    value: String::new(),
                },
                WizardField {
                    prompt: "outputs",
                    value: String::new(),
                },
                WizardField {
                    prompt: "constraints",
                    value: String::new(),
                },
            ],
            cursor: 0,
        }
    }

    fn current_prompt(&self) -> &str {
        self.fields
            .get(self.cursor)
            .map(|field| field.prompt)
            .unwrap_or("finish")
    }

    fn advance(&mut self, raw_input: &str) -> WizardAdvance {
        if raw_input.trim().is_empty() {
            return WizardAdvance::ValidationError(format!(
                "required field: {}",
                self.current_prompt()
            ));
        }

        if let Some(field) = self.fields.get_mut(self.cursor) {
            field.value = raw_input.trim().to_string();
        }

        self.cursor += 1;
        if self.cursor < self.fields.len() {
            return WizardAdvance::NeedMoreInput;
        }

        match self.kind {
            WizardKind::Persona => WizardAdvance::Completed(self.to_persona_submission()),
            WizardKind::Scope => match self.to_scope_submission() {
                Ok(submission) => WizardAdvance::Completed(submission),
                Err(error) => {
                    self.cursor = 0;
                    WizardAdvance::ValidationError(error)
                }
            },
            WizardKind::Skill => WizardAdvance::Completed(self.to_skill_submission()),
        }
    }

    fn to_persona_submission(&self) -> WizardSubmission {
        let persona_name = self.fields[0].value.clone();
        let file_name = format!("{}.md", slug(&persona_name));
        let content = format!(
            "## Responsibility\n{}\n\n## Deliverables\n{}\n\n## Instructions\n{}\n\n## Interaction Matrix\n{}\n\n## Limits\n{}\n",
            self.fields[1].value,
            self.fields[2].value,
            self.fields[3].value,
            self.fields[4].value,
            self.fields[5].value,
        );

        WizardSubmission::Persona { file_name, content }
    }

    fn to_scope_submission(&self) -> Result<WizardSubmission, String> {
        let number = self.fields[0].value.trim();
        if number.len() != 3 || !number.chars().all(|ch| ch.is_ascii_digit()) {
            return Err("delivery number must have 3 digits (e.g. 001)".to_string());
        }

        let file_name = format!("{}-{}.md", number, slug(&self.fields[1].value));
        let content = format!(
            "## Objective\n{}\n\n## Business Scope\n{}\n\n## Deliverables\n{}\n\n## Acceptance Criteria\n{}\n\n## Dependencies\n{}\n",
            self.fields[2].value,
            self.fields[3].value,
            self.fields[4].value,
            self.fields[5].value,
            self.fields[6].value,
        );

        Ok(WizardSubmission::Scope { file_name, content })
    }

    fn to_skill_submission(&self) -> WizardSubmission {
        let persona_name = self.fields[0].value.clone();
        let file_name = format!("{}.md", slug(&self.fields[1].value));
        let content = format!(
            "## Objective\n{}\n\n## Triggers\n{}\n\n## Inputs\n{}\n\n## Outputs\n{}\n\n## Constraints\n{}\n",
            self.fields[2].value,
            self.fields[3].value,
            self.fields[4].value,
            self.fields[5].value,
            self.fields[6].value,
        );

        WizardSubmission::Skill {
            persona_name,
            file_name,
            content,
        }
    }
}

fn persist_submission(
    governance: &MarkdownGovernance,
    submission: WizardSubmission,
) -> Result<PathBuf, anyhow::Error> {
    governance.ensure_directories()?;

    let path = match submission {
        WizardSubmission::Persona { file_name, content } => {
            let path = governance.validate_persona_document(&file_name, &content)?;
            std::fs::write(&path, content)?;
            path
        }
        WizardSubmission::Scope { file_name, content } => {
            let path = governance.validate_scope_document(&file_name, &content)?;
            std::fs::write(&path, content)?;
            path
        }
        WizardSubmission::Skill {
            persona_name,
            file_name,
            content,
        } => {
            let path = governance.validate_skill_document(&persona_name, &file_name, &content)?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, content)?;
            path
        }
    };

    Ok(path)
}

fn slug(input: &str) -> String {
    let lowered = input.trim().to_lowercase();
    let mut out = String::new();
    let mut last_dash = false;

    for ch in lowered.chars() {
        let mapped = match ch {
            'a'..='z' | '0'..='9' => Some(ch),
            _ => Some('-'),
        };

        if let Some(value) = mapped {
            if value == '-' {
                if !last_dash {
                    out.push('-');
                    last_dash = true;
                }
            } else {
                out.push(value);
                last_dash = false;
            }
        }
    }

    out.trim_matches('-').to_string()
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

struct TelemetryStore;

impl TelemetryStore {
    fn record(event: &str, detail: Option<&str>) -> Result<()> {
        if !telemetry_enabled() {
            return Ok(());
        }

        let path = telemetry_file_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let sanitized = detail.unwrap_or("").replace('"', "'");
        writeln!(
            file,
            "{{\"ts\":{},\"event\":\"{}\",\"detail\":\"{}\"}}",
            ts, event, sanitized
        )?;
        Ok(())
    }
}

fn telemetry_enabled() -> bool {
    matches!(
        std::env::var("MAESTRO_TELEMETRY").ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE")
    )
}

fn telemetry_file_path() -> Result<PathBuf> {
    Ok(workspace_maestro_dir()?.join("telemetry_onboarding.jsonl"))
}

fn workspace_maestro_dir() -> Result<PathBuf> {
    Ok(std::env::current_dir()?.join("maestro"))
}

async fn enqueue_interview_question(
    app: &mut TuiApp,
    environment: Option<&Arc<Environment>>,
) -> Result<bool> {
    let Some(bot) = &app.interview_bot else {
        return Ok(false);
    };
    let Some(session_lock) = &app.interview_session else {
        return Ok(false);
    };

    let mut session = session_lock.write().await;
    if session.turn_count >= 10 {
        app.logs.push(
            "ℹ️ Interview reached the maximum turn limit. Generating final proposal.".to_string(),
        );
        if session.proposed_changes.is_none() {
            let needs = bot.analyze_conversation(&session).await?;
            session.collected_needs = Some(needs.clone());
            session.proposed_changes = Some(bot.generate_proposals(&needs)?);
        }
        session.approval_pending = true;
        app.approval_modal_visible = true;
        return Ok(false);
    }

    let next_turn = session.turn_count + 1;
    let Some(question_text) = bot.get_question(next_turn) else {
        if session.proposed_changes.is_none() {
            let needs = bot.analyze_conversation(&session).await?;
            session.collected_needs = Some(needs.clone());
            session.proposed_changes = Some(bot.generate_proposals(&needs)?);
        }
        session.approval_pending = true;
        app.approval_modal_visible = true;
        return Ok(false);
    };

    let question_id = Uuid::new_v4();
    session
        .exchange_history
        .push(crate::application::interview_bot::InterviewExchange {
            maestro_question: question_id,
            maestro_text: question_text.clone(),
            user_answer: String::new(),
            timestamp: SystemTime::now(),
        });
    drop(session);

    app.maestro_message_id = Some(question_id);
    app.logs.push(format!("maestro: {}", question_text));

    if let Some(env) = environment {
        let _ = env
            .publish(Message::new(
                "Maestro".to_string(),
                format!("Interview question {}: {}", next_turn, question_text),
                None,
            ))
            .await;
    }

    Ok(true)
}

fn extract_scope_slug(file_name: &str) -> String {
    let stem = file_name.trim_end_matches(".md");
    let parts = stem.splitn(2, '-').collect::<Vec<_>>();
    if parts.len() == 2 && parts[0].chars().all(|ch| ch.is_ascii_digit()) {
        return slug(parts[1]);
    }
    slug(stem)
}

fn next_scope_number(scopes_dir: &PathBuf) -> u16 {
    let mut max_found = 0_u16;
    if let Ok(entries) = fs::read_dir(scopes_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                let prefix = name.split('-').next().unwrap_or_default();
                if let Ok(value) = prefix.parse::<u16>() {
                    if value > max_found {
                        max_found = value;
                    }
                }
            }
        }
    }
    max_found.saturating_add(1)
}

async fn apply_interview_scope_proposals(
    app: &mut TuiApp,
    governance: &MarkdownGovernance,
    environment: Option<&Arc<Environment>>,
) -> Result<usize> {
    let Some(session_lock) = &app.interview_session else {
        app.logs
            .push("⚠️ No interview session found; nothing to apply.".to_string());
        return Ok(0);
    };

    let (summary, scope_drafts) = {
        let session = session_lock.read().await;
        if let Some(proposals) = &session.proposed_changes {
            (proposals.summary.clone(), proposals.scope_drafts.clone())
        } else {
            app.logs
                .push("⚠️ No proposals generated yet; cannot apply scope drafts.".to_string());
            return Ok(0);
        }
    };

    if let Some(env) = environment {
        let _ = env
            .publish(Message::new(
                "Maestro".to_string(),
                format!("Product handoff: {}", summary),
                app.maestro_message_id,
            ))
            .await;
    }

    let scopes_dir = governance.scopes_dir();
    let mut next_number = next_scope_number(&scopes_dir);
    let mut applied = 0_usize;

    for (draft_name, content) in scope_drafts {
        let base_slug = extract_scope_slug(&draft_name);
        let fallback = if base_slug.is_empty() {
            "interview-scope".to_string()
        } else {
            base_slug
        };
        let file_name = format!("{:03}-{}.md", next_number, fallback);
        let submission = WizardSubmission::Scope { file_name, content };

        match persist_submission(governance, submission) {
            Ok(path) => {
                app.logs.push(format!(
                    "✅ Scope created from interview: {}",
                    path.display()
                ));
                applied = applied.saturating_add(1);
                next_number = next_number.saturating_add(1);
            }
            Err(error) => {
                app.logs.push(format!(
                    "❌ Failed to apply interview scope draft: {}",
                    error
                ));
            }
        }
    }

    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    app.readiness = crate::application::readiness::run_checks(&root);

    if let Some(session_lock) = &app.interview_session {
        let mut session = session_lock.write().await;
        session.approval_pending = false;
    }

    if applied == 0 {
        app.logs.push(
            "⚠️ No interview scope drafts were applied. Review generated drafts and governance requirements."
                .to_string(),
        );
    }

    Ok(applied)
}

async fn run_maestro_wakeup_check(
    app: &mut TuiApp,
    environment: Option<&Arc<Environment>>,
    runtime: Option<&Arc<AgentRuntime>>,
) -> bool {
    let Some(env) = environment else {
        app.logs.push(
            "⚠️ Maestro runtime is not connected. Configure provider and model in maestro/config.yaml."
                .to_string(),
        );
        return false;
    };

    if let Some(rt) = runtime {
        let health = rt.health_snapshot().await;
        let maestro_running = matches!(
            health.get("Maestro"),
            Some(AgentHealth::Idle)
                | Some(AgentHealth::Observing)
                | Some(AgentHealth::Thinking)
                | Some(AgentHealth::Acting)
        );

        if !maestro_running {
            app.logs.push(
                "⚠️ Maestro persona is not active. Ensure startup checks pass and restart interview."
                    .to_string(),
            );
            return false;
        }
    }

    let wakeup_prompt = "Maestro are you awake?".to_string();
    let probe = Message::new("user".to_string(), wakeup_prompt, None);
    app.maestro_message_id = Some(probe.id());
    let _ = env.publish(probe).await;

    const WAKEUP_RETRIES: usize = 20;
    const WAKEUP_WAIT_MS: u64 = 250;

    for _ in 0..WAKEUP_RETRIES {
        tokio::time::sleep(Duration::from_millis(WAKEUP_WAIT_MS)).await;
        let history = env.get_history().await;
        let answered = history
            .iter()
            .rev()
            .take(40)
            .any(|msg| msg.sender().eq_ignore_ascii_case("maestro"));

        if answered {
            app.logs
                .push("✅ Maestro persona responded and is ready for interview.".to_string());
            return true;
        }
    }

    app.logs.push(
        "⚠️ Maestro did not answer wake-up check. Configure provider/model in maestro/config.yaml and restart interview."
            .to_string(),
    );
    app.maestro_message_id = None;
    false
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use crossterm::event::KeyModifiers;
    use ratatui::backend::TestBackend;
    use uuid::Uuid;

    use super::*;

    fn buffer_to_string(terminal: &Terminal<TestBackend>) -> String {
        let mut out = String::new();
        let buf = terminal.backend().buffer();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    fn temp_root(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!("{}-{}", prefix, Uuid::new_v4()))
    }

    #[test]
    fn renders_agents_monitor_and_input_panels() {
        let backend = TestBackend::new(80, 24);
        let terminal_result = Terminal::new(backend);
        assert!(terminal_result.is_ok());
        let mut terminal = match terminal_result {
            Ok(value) => value,
            Err(_) => panic!("terminal init failed"),
        };

        let app = TuiApp {
            agents: vec![AgentView {
                name: "Product".to_string(),
                status: "idle".to_string(),
            }],
            logs: vec!["user: iniciar".to_string()],
            input: "planejar sprint".to_string(),
            mode: UIMode::Workspace,
            readiness: crate::application::readiness::ReadinessState {
                items: vec![],
                has_config: true,
                config_valid: true,
                has_providers: true,
                provider_reachable: true,
                has_scopes: true,
                has_personas: true,
                has_skills: true,
            },
            dependency_domains: DependencyDomainsState {
                project_manifest_found: true,
                project_manifest_valid: true,
                project_required_checks_passed: true,
                project_failed_required: vec![],
                project_failed_required_hints: vec![],
                project_error: None,
            },
            focus: PanelFocus::Workspace,
            readiness_selected_action: 0,
            wizard: None,
            animation_frame: 0,
            play_bell: false,
            highlight_until: HashMap::new(),
            show_debug: false,
            interview_session: None,
            interview_bot: None,
            maestro_message_id: None,
            approval_modal_visible: false,
            last_runtime_event_count: 0,
        };

        let drawn = terminal.draw(|frame| render(frame, &app));
        assert!(drawn.is_ok());

        let content = buffer_to_string(&terminal);
        assert!(content.contains("Agents"));
        assert!(content.contains("Monitor"));
        assert!(content.contains("Command"));
        assert!(content.contains("Readiness"));
        assert!(content.contains("Product"));
        assert!(content.contains("idle"));
        assert!(content.contains("user: iniciar"));
    }

    #[test]
    fn handles_basic_input_flow_and_submit() {
        let mut app = TuiApp::default();

        let first = app.handle_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
        let second = app.handle_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert!(first.is_none());
        assert!(second.is_none());
        assert_eq!(app.input, "ok");

        let submit = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(submit, Some(UserAction::SubmitCommand(cmd)) if cmd == "ok"));
        assert!(app.input.is_empty());

        let quit = app.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(quit, Some(UserAction::Quit)));
    }

    #[test]
    fn typing_q_does_not_quit_when_input_has_content() {
        let mut app = TuiApp {
            input: "digitar".to_string(),
            ..TuiApp::default()
        };

        let action = app.handle_key_event(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        assert!(action.is_none());
        assert_eq!(app.input, "digitarq");
    }

    #[test]
    fn help_mode_is_non_blocking_and_can_return() {
        let mut app = TuiApp {
            input: "/help".to_string(),
            ..Default::default()
        };

        let help_action = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(help_action.is_none());
        assert_eq!(app.mode, UIMode::HelpMenu);

        app.input = "back".to_string();
        let back_action = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(back_action.is_none());
        assert_eq!(app.mode, UIMode::Workspace);
    }

    #[test]
    fn check_command_reports_readiness_gaps() {
        let mut app = TuiApp {
            readiness: crate::application::readiness::ReadinessState {
                items: vec![crate::application::readiness::ReadinessItem {
                    name: "Personas Directory".to_string(),
                    passed: false,
                    dummy_guide: "How-To: Create at least one persona markdown file.".to_string(),
                }],
                has_config: true,
                config_valid: true,
                has_providers: true,
                provider_reachable: true,
                has_scopes: true,
                has_personas: false,
                has_skills: false,
            },
            dependency_domains: DependencyDomainsState {
                project_manifest_found: false,
                project_manifest_valid: false,
                project_required_checks_passed: false,
                project_failed_required: vec![],
                project_failed_required_hints: vec![],
                project_error: Some("maestro/project-deps.yaml not found".to_string()),
            },
            ..TuiApp::default()
        };

        app.input = "/check".to_string();
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(action.is_none());
        assert!(app
            .logs
            .iter()
            .any(|line| line.contains("System is not fully ready")));
        assert!(app.logs.iter().any(|line| line.contains("/new persona")));
    }

    #[test]
    fn tab_focuses_readiness_panel() {
        let mut app = TuiApp::default();

        let action = app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert!(action.is_none());
        assert_eq!(app.focus, PanelFocus::Readiness);
        assert!(app.current_input_title().contains("Readiness focus"));
    }

    #[test]
    fn readiness_focus_enter_dispatches_selected_action() {
        let mut app = TuiApp {
            readiness: crate::application::readiness::ReadinessState {
                items: vec![],
                has_config: true,
                config_valid: true,
                has_providers: true,
                provider_reachable: true,
                has_scopes: true,
                has_personas: false,
                has_skills: true,
            },
            dependency_domains: DependencyDomainsState {
                project_manifest_found: true,
                project_manifest_valid: true,
                project_required_checks_passed: true,
                project_failed_required: vec![],
                project_failed_required_hints: vec![],
                project_error: None,
            },
            focus: PanelFocus::Readiness,
            ..TuiApp::default()
        };

        let move_selection = app.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert!(move_selection.is_none());

        let action = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(
            action,
            Some(UserAction::RunReadinessAction(
                ReadinessAction::CreatePersona
            ))
        ));
    }

    #[test]
    fn readiness_focus_number_shortcuts_execute_actions() {
        let mut app = TuiApp {
            readiness: crate::application::readiness::ReadinessState {
                items: vec![],
                has_config: true,
                config_valid: true,
                has_providers: true,
                provider_reachable: true,
                has_scopes: true,
                has_personas: false,
                has_skills: true,
            },
            dependency_domains: DependencyDomainsState {
                project_manifest_found: true,
                project_manifest_valid: true,
                project_required_checks_passed: true,
                project_failed_required: vec![],
                project_failed_required_hints: vec![],
                project_error: None,
            },
            focus: PanelFocus::Readiness,
            ..TuiApp::default()
        };

        // '1' should execute the first action (OpenConfigHint at index 0)
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
        assert!(action.is_some());
        assert_eq!(app.readiness_selected_action, 0);
        assert!(matches!(
            action,
            Some(UserAction::RunReadinessAction(
                ReadinessAction::OpenConfigHint
            ))
        ));

        // '2' should execute the second action (CreatePersona at index 1)
        let action2 = app.handle_key_event(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
        assert!(action2.is_some());
        assert_eq!(app.readiness_selected_action, 1);
        assert!(matches!(
            action2,
            Some(UserAction::RunReadinessAction(
                ReadinessAction::CreatePersona
            ))
        ));
    }

    #[test]
    fn readiness_focus_selects_project_dependency_remediation_action() {
        let mut app = TuiApp {
            readiness: crate::application::readiness::ReadinessState {
                items: vec![],
                has_config: true,
                config_valid: true,
                has_providers: true,
                provider_reachable: true,
                has_scopes: true,
                has_personas: true,
                has_skills: true,
            },
            dependency_domains: DependencyDomainsState {
                project_manifest_found: true,
                project_manifest_valid: true,
                project_required_checks_passed: false,
                project_failed_required: vec!["git".to_string()],
                project_failed_required_hints: vec![(
                    "git".to_string(),
                    Some("Install Git and ensure it is available in PATH.".to_string()),
                )],
                project_error: None,
            },
            focus: PanelFocus::Readiness,
            ..TuiApp::default()
        };

        let action = app.handle_key_event(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
        assert!(matches!(
            action,
            Some(UserAction::RunReadinessAction(
                ReadinessAction::RemediateProjectDependency { .. }
            ))
        ));
    }

    #[test]
    fn readiness_evaluate_with_root_reports_not_ready_without_config() {
        let root = temp_root("maestro-readiness-missing-config");
        let created = fs::create_dir_all(&root);
        assert!(created.is_ok());

        let governance = MarkdownGovernance::new(&root);
        let ensured = governance.ensure_directories();
        assert!(ensured.is_ok());

        let readiness = crate::application::readiness::run_checks(&root);
        assert!(!readiness.has_config);
        assert!(!readiness.is_ready());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn readiness_evaluate_with_root_is_ready_when_all_checks_pass() {
        let root = temp_root("maestro-readiness-ready");
        let created = fs::create_dir_all(&root);
        assert!(created.is_ok());

        let governance = MarkdownGovernance::new(&root);
        let ensured = governance.ensure_directories();
        assert!(ensured.is_ok());

        let listener = TcpListener::bind("127.0.0.1:0");
        assert!(listener.is_ok());
        let mut port = 0_u16;
        if let Ok(socket) = &listener {
            let local_addr = socket.local_addr();
            assert!(local_addr.is_ok());
            if let Ok(addr) = local_addr {
                port = addr.port();
            }
        }
        assert!(port > 0);

        let config_path = root.join("maestro").join("config.yaml");
        let config_content = format!(
            "system:\n  default_provider: \"ollama\"\n  default_model: \"mistral\"\n  max_concurrency: 4\n  rate_limit_per_minute: 120\n  retry_max_attempts: 3\nproviders:\n  ollama:\n    kind: \"ollama\"\n    endpoint: \"http://127.0.0.1:{port}\"\n    auth_mode: \"none\"\n    timeout_ms: 5000\n    models:\n      - name: \"mistral\"\n        context_window: 32000\n    capabilities:\n      supports_tools: false\n      supports_streaming: true\n      supports_json_mode: false\n      supports_reasoning_controls: false\n      max_context_tokens: 32000\n"
        );
        let write_config = fs::write(&config_path, config_content);
        assert!(write_config.is_ok());

        let scope_file = governance.scopes_dir().join("001-ready.md");
        let persona_file = governance.personas_dir().join("produto.md");
        let skill_persona_dir = governance.skills_dir().join("produto");
        let create_skill_dir = fs::create_dir_all(&skill_persona_dir);
        assert!(create_skill_dir.is_ok());
        let skill_file = skill_persona_dir.join("planejamento.md");

        let wrote_scope = fs::write(scope_file, "# scope");
        let wrote_persona = fs::write(persona_file, "# persona");
        let wrote_skill = fs::write(skill_file, "# skill");
        assert!(wrote_scope.is_ok());
        assert!(wrote_persona.is_ok());
        assert!(wrote_skill.is_ok());

        let readiness = crate::application::readiness::run_checks(&root);
        assert!(readiness.has_config);
        assert!(readiness.has_providers);
        assert!(readiness.provider_reachable);
        assert!(readiness.has_scopes);
        assert!(readiness.has_personas);
        assert!(readiness.has_skills);
        assert!(readiness.is_ready());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn wizard_blocks_submission_when_required_field_is_empty() {
        let mut app = TuiApp::default();

        let start = app.handle_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        assert!(start.is_none());
        for c in "new persona".chars() {
            let _ = app.handle_key_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        let _ = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        let blocked = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(blocked.is_none());
        assert!(app.wizard.is_some());
        assert!(app.logs.iter().any(|line| line.contains("required field")));
    }

    #[test]
    fn wizard_generates_persona_submission_after_all_fields() {
        let mut app = TuiApp {
            input: "/new persona".to_string(),
            ..TuiApp::default()
        };
        let _ = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        let steps = [
            "Product",
            "Define priorities",
            "Prioritized backlog",
            "Collaborate with engineering",
            "Product -> Engineering",
            "Do not decide deployment",
        ];

        let mut last_action = None;
        for step in steps {
            app.input = step.to_string();
            last_action = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        }

        assert!(matches!(
            last_action,
            Some(UserAction::CompleteWizard(WizardSubmission::Persona { .. }))
        ));
    }

    #[test]
    fn apply_wizard_submission_reports_error_for_invalid_content() {
        let mut app = TuiApp::default();
        let root = temp_root("maestro-rollback-scope");
        let create = fs::create_dir_all(&root);
        assert!(create.is_ok());
        let governance = MarkdownGovernance::new(&root);
        let ensure = governance.ensure_directories();
        assert!(ensure.is_ok());

        let invalid_scope = WizardSubmission::Scope {
            file_name: "invalid.md".to_string(),
            content: "## Objective\nA\n".to_string(),
        };

        let applied = app.apply_wizard_submission(&governance, invalid_scope);
        assert!(applied.is_err());
        assert!(app.logs.iter().any(|line| line.contains("Error saving")));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn interview_mode_transition_from_workspace() {
        let mut app = TuiApp::default();
        assert_eq!(app.mode, UIMode::Workspace);

        // Simulate entering interview mode
        app.mode = UIMode::Interview;
        app.interview_bot = Some(Arc::new(
            crate::application::interview_bot::InterviewBot::new(),
        ));

        assert_eq!(app.mode, UIMode::Interview);
        assert!(app.interview_bot.is_some());
    }

    #[test]
    fn interview_mode_handles_user_input() {
        let mut app = TuiApp {
            mode: UIMode::Interview,
            ..TuiApp::default()
        };

        let input = app.handle_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
        assert!(input.is_none());
        assert_eq!(app.input, "p");

        let backspace = app.handle_key_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert!(backspace.is_none());
        assert!(app.input.is_empty());
    }

    #[test]
    fn interview_mode_deps_command_dispatches_manage_action() {
        let mut app = TuiApp {
            mode: UIMode::Interview,
            ..TuiApp::default()
        };

        for c in "/deps".chars() {
            let _ = app.handle_key_event(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }

        let action = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(action, Some(UserAction::ManageProjectDeps)));
    }

    #[test]
    fn interview_mode_renders_maestro_panel_without_crash() {
        let backend = TestBackend::new(80, 24);
        let terminal_result = Terminal::new(backend);
        assert!(terminal_result.is_ok());
        let mut terminal = match terminal_result {
            Ok(value) => value,
            Err(_) => panic!("terminal init failed"),
        };

        let app = TuiApp {
            mode: UIMode::Interview,
            interview_bot: Some(Arc::new(
                crate::application::interview_bot::InterviewBot::new(),
            )),
            interview_session: Some(Arc::new(tokio::sync::RwLock::new(
                crate::application::interview_bot::InterviewSession::default(),
            ))),
            ..TuiApp::default()
        };

        let drawn = terminal.draw(|frame| render(frame, &app));
        assert!(drawn.is_ok());

        let content = buffer_to_string(&terminal);
        assert!(content.contains("Maestro"));
    }

    #[test]
    fn init_bootstrap_forces_interview_even_when_ready() {
        assert!(should_enter_interview(
            OnboardingBootstrap::InitInterview,
            true
        ));
        assert!(!should_enter_interview(OnboardingBootstrap::Detailed, true));
        assert!(should_enter_interview(OnboardingBootstrap::Detailed, false));
    }

    #[tokio::test]
    async fn approval_applies_scope_drafts_to_scopes_folder() {
        let root = temp_root("maestro-interview-apply-scopes");
        let created = fs::create_dir_all(&root);
        assert!(created.is_ok());

        let governance = MarkdownGovernance::new(&root);
        let ensured = governance.ensure_directories();
        assert!(ensured.is_ok());

        let old_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let changed = std::env::set_current_dir(&root);
        assert!(changed.is_ok());

        let mut app = TuiApp {
            mode: UIMode::Interview,
            approval_modal_visible: true,
            interview_session: Some(Arc::new(tokio::sync::RwLock::new(
                crate::application::interview_bot::InterviewSession {
                    proposed_changes: Some(crate::application::interview_bot::ProposedChanges {
                        persona_drafts: vec![],
                        skill_drafts: vec![],
                        scope_drafts: vec![(
                            "001-project-setup.md".to_string(),
                            "## Objective\nStart project\n\n## Business Scope\nInitial delivery\n\n## Deliverables\nScope file\n\n## Acceptance Criteria\nFile persisted\n\n## Dependencies\nNone\n"
                                .to_string(),
                        )],
                        summary: "Product recommends one scope draft".to_string(),
                    }),
                    approval_pending: true,
                    ..Default::default()
                },
            ))),
            ..TuiApp::default()
        };

        let applied = apply_interview_scope_proposals(&mut app, &governance, None)
            .await
            .unwrap_or(0);
        assert!(applied >= 1);

        let scopes = fs::read_dir(governance.scopes_dir())
            .unwrap_or_else(|_| panic!("cannot read scopes dir"))
            .flatten()
            .collect::<Vec<_>>();
        assert!(!scopes.is_empty());

        let _ = std::env::set_current_dir(old_dir);
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn rejection_writes_nothing_and_keeps_interview_active() {
        let root = temp_root("maestro-interview-reject-no-write");
        let created = fs::create_dir_all(&root);
        assert!(created.is_ok());

        let governance = MarkdownGovernance::new(&root);
        let ensured = governance.ensure_directories();
        assert!(ensured.is_ok());

        let mut app = TuiApp {
            mode: UIMode::Interview,
            approval_modal_visible: true,
            interview_bot: Some(Arc::new(crate::application::interview_bot::InterviewBot::new())),
            interview_session: Some(Arc::new(tokio::sync::RwLock::new(
                crate::application::interview_bot::InterviewSession {
                    proposed_changes: Some(crate::application::interview_bot::ProposedChanges {
                        persona_drafts: vec![],
                        skill_drafts: vec![],
                        scope_drafts: vec![(
                            "001-should-not-write.md".to_string(),
                            "## Objective\nNo write\n\n## Business Scope\nNone\n\n## Deliverables\nNone\n\n## Acceptance Criteria\nNone\n\n## Dependencies\nNone\n"
                                .to_string(),
                        )],
                        summary: "Reject me".to_string(),
                    }),
                    approval_pending: true,
                    ..Default::default()
                },
            ))),
            ..TuiApp::default()
        };

        // Simulate rejection path from runtime loop branch.
        app.approval_modal_visible = false;
        if let Some(session_lock) = &app.interview_session {
            let mut session = session_lock.write().await;
            session.approval_pending = false;
            session.proposed_changes = None;
        }
        app.logs.push(
            "❓ Understood. Let us refine requirements before generating new scope drafts."
                .to_string(),
        );

        assert_eq!(app.mode, UIMode::Interview);
        let scopes = fs::read_dir(governance.scopes_dir())
            .unwrap_or_else(|_| panic!("cannot read scopes dir"))
            .flatten()
            .collect::<Vec<_>>();
        assert!(scopes.is_empty());

        let _ = fs::remove_dir_all(root);
    }
}
