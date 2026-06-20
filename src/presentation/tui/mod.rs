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
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table};
use ratatui::{Frame, Terminal};
use tui_big_text::{BigText, PixelSize};

use crate::application::agent_runtime::{AgentHealth, AgentRuntime};
use crate::application::config::DEFAULT_CONFIG_TEMPLATE;
use crate::application::environment::Environment;
use crate::application::markdown_governance::MarkdownGovernance;
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum PanelFocus {
    #[default]
    Workspace,
    Readiness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadinessAction {
    CreateConfigTemplate,
    OpenConfigHint,
    ConfigureProviders,
    StartProvider,
    CreateScope,
    CreatePersona,
    CreateSkill,
}

impl ReadinessAction {
    fn label(&self) -> &'static str {
        match self {
            ReadinessAction::CreateConfigTemplate => "Create config template",
            ReadinessAction::OpenConfigHint => "Open config guidance",
            ReadinessAction::ConfigureProviders => "Configure providers",
            ReadinessAction::StartProvider => "Start provider",
            ReadinessAction::CreateScope => "Create scope",
            ReadinessAction::CreatePersona => "Create persona",
            ReadinessAction::CreateSkill => "Create skill",
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
    focus: PanelFocus,
    readiness_selected_action: usize,
    wizard: Option<CreationWizard>,
    animation_frame: usize,
    play_bell: bool,
    highlight_until: HashMap<String, usize>,
    pub show_debug: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum OnboardingBootstrap {
    Auto,
    UserIntro,
    ProjectSetup,
}

impl TuiApp {
    pub fn with_readiness(
        mut self,
        readiness: crate::application::readiness::ReadinessState,
    ) -> Self {
        self.readiness = readiness;
        self
    }

    pub fn show_help(&mut self) {
        self.mode = UIMode::HelpMenu;
        self.logs.clear();
        self.logs.push("📚 MAESTRO - MANUAL INTERATIVO".to_string());
        self.logs.push(String::new());
        self.logs.push("Start (Quick Start):".to_string());
        self.logs
            .push("  /new persona   - Criar nova persona (IA agent)".to_string());
        self.logs
            .push("  /new scope     - Criar novo escopo de trabalho".to_string());
        self.logs
            .push("  /new skill     - Ensinar nova skill a um agent".to_string());
        self.logs.push(String::new());
        self.logs.push("Consultar Status:".to_string());
        self.logs
            .push("  /status        - View agents health".to_string());
        self.logs
            .push("  /check         - Check if system is ready".to_string());
        self.logs.push(String::new());
        self.logs
            .push("Edit Configurations (in text editor):".to_string());
        self.logs
            .push("  maestro/config.toml       - Configurar providers/modelos".to_string());
        self.logs
            .push("  maestro/personas/*.md     - Editar personas livremente".to_string());
        self.logs
            .push("  maestro/scopes/*.md       - Editar escopos de trabalho".to_string());
        self.logs.push(String::new());
        self.logs.push("Controles:".to_string());
        self.logs
            .push("  Ctrl+L         - Logout de provedores (ex: Google Gemini)".to_string());
        self.logs
            .push("  Ctrl+D         - Toggle debug panel".to_string());
        self.logs
            .push("  q              - Sair (quando input vazio)".to_string());
        self.logs.push("  ESC            - Sair".to_string());
        self.logs.push(String::new());
        self.logs
            .push("Digite 'back' para retornar ao workspace".to_string());
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
        self.normalize_readiness_selection();
    }

    fn readiness_actions(&self) -> Vec<ReadinessAction> {
        let mut actions = Vec::new();

        if !self.readiness.has_config {
            actions.push(ReadinessAction::CreateConfigTemplate);
        } else {
            actions.push(ReadinessAction::OpenConfigHint);
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
        actions.get(self.readiness_selected_action).copied()
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
                let config_path = root.join("maestro").join("config.toml");
                if !config_path.exists() {
                    if let Some(parent) = config_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if fs::write(&config_path, DEFAULT_CONFIG_TEMPLATE).is_ok() {
                        self.logs.push(format!(
                            "readiness action: config template criado em {}",
                            config_path.display()
                        ));
                    } else {
                        self.logs
                            .push("readiness action: falha ao criar config template".to_string());
                    }
                } else {
                    self.logs
                        .push("readiness action: config ja existe".to_string());
                }
            }
            ReadinessAction::OpenConfigHint => {
                self.logs
                    .push("readiness action: abra maestro/config.toml no editor".to_string());
            }
            ReadinessAction::ConfigureProviders => {
                self.logs.push(
                    "readiness action: adicione [[providers]] e runtime.default_provider em maestro/config.toml"
                        .to_string(),
                );
            }
            ReadinessAction::StartProvider => {
                self.logs.push(
                    "readiness action: inicie o provider default (ex: ollama serve)".to_string(),
                );
            }
            ReadinessAction::CreateScope => {
                self.wizard = Some(CreationWizard::new_scope());
                self.focus = PanelFocus::Workspace;
                self.logs
                    .push("readiness action: wizard de scope iniciado".to_string());
            }
            ReadinessAction::CreatePersona => {
                self.wizard = Some(CreationWizard::new_persona());
                self.focus = PanelFocus::Workspace;
                self.logs
                    .push("readiness action: wizard de persona iniciado".to_string());
            }
            ReadinessAction::CreateSkill => {
                self.wizard = Some(CreationWizard::new_skill());
                self.focus = PanelFocus::Workspace;
                self.logs
                    .push("readiness action: wizard de skill iniciado".to_string());
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
        use crate::application::agent_observability::RuntimeEvent;

        let lines = events
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

        // Keep only last 100 lines
        let keep_lines = if lines.len() > 100 {
            lines.split_at(lines.len() - 100).1.to_vec()
        } else {
            lines
        };

        self.logs = keep_lines;
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
                            self.logs.push(format!("wizard iniciado: {command}"));
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
                        "✅ PRONTO PARA VOAR! Sistema configurado e personalizavel."
                    } else {
                        "⚠️  Sistema nao esta totalmente pronto.\n\nPasso a passo sugerido:\n"
                    };
                    self.logs.push(status.to_string());

                    if !self.readiness.has_config {
                        self.logs.push(
                            "  1. Criar maestro/config.toml (use: maestro init-config)".to_string(),
                        );
                    }
                    if self.readiness.has_config && !self.readiness.has_providers {
                        self.logs.push(
                            "  2. Definir pelo menos um [[providers]] valido no config.toml"
                                .to_string(),
                        );
                    }
                    if self.readiness.has_providers && !self.readiness.provider_reachable {
                        self.logs.push(
                            "  3. Subir o provider default (ex: ollama serve) ou ajustar endpoint"
                                .to_string(),
                        );
                    }
                    if !self.readiness.has_scopes {
                        self.logs
                            .push("  4. Criar um scope: /new scope".to_string());
                    }
                    if !self.readiness.has_personas {
                        self.logs
                            .push("  5. Criar uma persona: /new persona".to_string());
                    }
                    if !self.readiness.has_skills {
                        self.logs
                            .push("  6. Criar uma skill: /new skill".to_string());
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
            _ => Err("tipo de wizard invalido: use persona, scope ou skill".to_string()),
        }
    }

    fn current_input_title(&self) -> String {
        if let Some(wizard) = &self.wizard {
            format!(
                "Wizard {} - {} (Enter confirma, q sai)",
                wizard.kind.label(),
                wizard.current_prompt()
            )
        } else if self.focus == PanelFocus::Readiness {
            "Readiness focus (Tab alterna, Up/Down seleciona, Enter executa)".to_string()
        } else if self.mode == UIMode::HelpMenu {
            "Help (escreva 'back' para retornar)".to_string()
        } else {
            "Comando (Enter envia, q sai | /help /check /new persona|scope|skill)".to_string()
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
                    .push(format!("✅ Arquivo criado: {}", path.display()));
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
                .push("✅ Auto-configured maestro/config.toml (Ollama detected!)".to_string());
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

            // Also display runtime events for observability
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
                                let message = Message::new("user".to_string(), command, None);
                                if let Some(env) = &environment { let _ = env.publish(message).await; }
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

    // Workspace Area: Logo on top, Agentes and Logs side-by-side
    let ws_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Logo
            Constraint::Min(0),    // Agentes and Logs
        ])
        .split(workspace_area);

    render_logo_panel(frame, ws_rows[0]);

    let ws_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40), // Agentes
            Constraint::Percentage(60), // Logs
        ])
        .split(ws_rows[1]);

    render_agents_panel(frame, ws_cols[0], app);
    render_monitor_panel(frame, ws_cols[1], app);

    // Bottom Area
    let agentes_width = ws_cols[0].width;

    let bottom_cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(agentes_width), // Agentes Destacados
            Constraint::Min(0),                // Comando (Largura da tela - Agentes Destacados)
        ])
        .split(bottom_area);

    render_gauge_panel(frame, bottom_cols[0], app);

    // Comando altura = metade de Agentes Destacados
    let command_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(0),   // Espaço vazio no topo
            Constraint::Percentage(100), // Comando na metade inferior
        ])
        .split(bottom_cols[1]);

    render_input_panel(frame, command_rows[1], app);
}

fn render_readiness_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect, app: &TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(10)])
        .split(area);

    let (headline, headline_style) = if app.readiness.is_ready() {
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

    for check in &app.readiness.items {
        lines.push(readiness_line(&check.name, check.passed));
    }

    let paragraph = Paragraph::new(lines.join("\n"))
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title("Readiness")
                .borders(Borders::ALL)
                .border_style(if app.readiness.is_ready() {
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

    if !app.readiness.is_ready() {
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
        .title("Agentes Destacados")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(218, 165, 32)));

    // Simplistic placeholder for the gauge in the mockup
    let paragraph = Paragraph::new("\n\n        [ Gauge 80% ]\n")
        .style(Style::default().fg(Color::White))
        .block(block);

    frame.render_widget(paragraph, area);
}

fn render_logo_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect) {
    let big_text = BigText::builder()
        .pixel_size(PixelSize::Full)
        .style(Style::default().fg(Color::Rgb(218, 165, 32))) // Dourado
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
    .header(Row::new(vec!["Agente", "Status"]).style(Style::default().fg(Color::Rgb(218, 165, 32))))
    .block(
        Block::default()
            .title("Agentes")
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

    // Show blinking cursor when input panel is in focus
    let is_focused = app.wizard.is_some() || app.focus == PanelFocus::Workspace;
    if is_focused {
        let max_x = area.x + area.width.saturating_sub(2); // Leave 1 cell for right border
        let cursor_x = (area.x + 1 + app.input.chars().count() as u16).min(max_x);
        let cursor_y = area.y + 1; // 1 cell down for top border

        frame.set_cursor_position((cursor_x, cursor_y));
    }
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
                    prompt: "nome da persona",
                    value: String::new(),
                },
                WizardField {
                    prompt: "responsabilidade",
                    value: String::new(),
                },
                WizardField {
                    prompt: "entregaveis",
                    value: String::new(),
                },
                WizardField {
                    prompt: "instrucoes",
                    value: String::new(),
                },
                WizardField {
                    prompt: "matriz de interacao",
                    value: String::new(),
                },
                WizardField {
                    prompt: "limites",
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
                    prompt: "numero da entrega (ex: 001)",
                    value: String::new(),
                },
                WizardField {
                    prompt: "nome da entrega",
                    value: String::new(),
                },
                WizardField {
                    prompt: "objetivo",
                    value: String::new(),
                },
                WizardField {
                    prompt: "escopo de negocio",
                    value: String::new(),
                },
                WizardField {
                    prompt: "entregaveis",
                    value: String::new(),
                },
                WizardField {
                    prompt: "criterios de aceite",
                    value: String::new(),
                },
                WizardField {
                    prompt: "dependencias",
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
                    prompt: "persona alvo",
                    value: String::new(),
                },
                WizardField {
                    prompt: "nome da skill",
                    value: String::new(),
                },
                WizardField {
                    prompt: "objetivo",
                    value: String::new(),
                },
                WizardField {
                    prompt: "gatilhos",
                    value: String::new(),
                },
                WizardField {
                    prompt: "entradas",
                    value: String::new(),
                },
                WizardField {
                    prompt: "saidas",
                    value: String::new(),
                },
                WizardField {
                    prompt: "restricoes",
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
            .unwrap_or("concluir")
    }

    fn advance(&mut self, raw_input: &str) -> WizardAdvance {
        if raw_input.trim().is_empty() {
            return WizardAdvance::ValidationError(format!(
                "campo obrigatorio: {}",
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
            "## Responsabilidade\n{}\n\n## Entregaveis\n{}\n\n## Instrucoes\n{}\n\n## Matriz de Interacao\n{}\n\n## Limites\n{}\n",
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
            return Err("numero da entrega deve ter 3 digitos (ex: 001)".to_string());
        }

        let file_name = format!("{}-{}.md", number, slug(&self.fields[1].value));
        let content = format!(
            "## Objetivo\n{}\n\n## Escopo de Negocio\n{}\n\n## Entregaveis\n{}\n\n## Criterios de Aceite\n{}\n\n## Dependencias\n{}\n",
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
            "## Objetivo\n{}\n\n## Gatilhos\n{}\n\n## Entradas\n{}\n\n## Saidas\n{}\n\n## Restricoes\n{}\n",
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
            focus: PanelFocus::Workspace,
            readiness_selected_action: 0,
            wizard: None,
            animation_frame: 0,
            play_bell: false,
            highlight_until: HashMap::new(),
            show_debug: false,
        };

        let drawn = terminal.draw(|frame| render(frame, &app));
        assert!(drawn.is_ok());

        let content = buffer_to_string(&terminal);
        assert!(content.contains("Agentes"));
        assert!(content.contains("Monitor"));
        assert!(content.contains("Comando"));
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
            ..TuiApp::default()
        };

        app.input = "/check".to_string();
        let action = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(action.is_none());
        assert!(app
            .logs
            .iter()
            .any(|line| line.contains("nao esta totalmente pronto")));
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

        let config_path = root.join("maestro").join("config.toml");
        let config_content = format!(
            "[[providers]]\nname = \"ollama\"\nendpoint = \"http://127.0.0.1:{port}/v1\"\nauth_mode = \"none\"\ntimeout_ms = 5000\nmodels = [\"deepseek-coder-v2\"]\nmax_context_chars = 128000\n\n[runtime]\nretry_max_attempts = 3\nmax_concurrency = 4\nrate_limit_per_minute = 120\ndefault_provider = \"ollama\"\ndefault_model = \"deepseek-coder-v2\"\n"
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
        assert!(app
            .logs
            .iter()
            .any(|line| line.contains("campo obrigatorio")));
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
            "Definir prioridades",
            "Backlog priorizado",
            "Trabalhar com engenharia",
            "Product -> Engineering",
            "Nao decidir deploy",
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
            content: "## Objetivo\nA\n## Escopo de Negocio\nB\n## Entregaveis\nC\n## Criterios de Aceite\nD\n## Dependencias\nE\n".to_string(),
        };

        let applied = app.apply_wizard_submission(&governance, invalid_scope);
        assert!(applied.is_err());
        assert!(app.logs.iter().any(|line| line.contains("Error saving")));

        let _ = fs::remove_dir_all(root);
    }
}
