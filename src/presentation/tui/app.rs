use super::*;

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
        self.logs.push(
            "  /architect     - Open Architect Mode directives hub (edit/archive)".to_string(),
        );
        self.logs
            .push("  /monitor       - Return to the runtime workspace monitor".to_string());
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
        self.architect_picker = None;
    }

    /// Enter Architect Mode (the directive select stage) and build the picker from disk.
    pub(super) fn enter_directive_select(&mut self, governance: &MarkdownGovernance) {
        let picker = ArchitectPicker::from_governance(governance);
        let count = picker.entries.len();
        self.architect_picker = Some(picker);
        self.mode = UIMode::Architect;
        self.logs.push(format!(
            "\u{1f4cb} Directives editor \u{2014} {count} directive(s). Up/Down navigate, Enter select, Esc back."
        ));
    }

    /// Resolve the directive selected in the Core picker, if any.
    ///
    /// Returns `None` when the selection targets the immutable Maestro persona,
    /// enforcing read-only at the presentation boundary (defense in depth).
    pub(super) fn architect_selection_target(
        &self,
    ) -> Option<crate::application::interview_bot::DirectiveTarget> {
        let entry = self.architect_picker.as_ref()?.selected()?;
        if entry.read_only {
            return None;
        }
        Some(entry.directive_target())
    }

    fn toggle_focus(&mut self) {
        self.focus = self.focus.next();
    }

    pub fn tick_animation(&mut self) {
        self.animation_frame = self.animation_frame.wrapping_add(1);
        self.normalize_readiness_selection();
    }

    pub(super) fn refresh_readiness(&mut self, _governance: &MarkdownGovernance) {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        self.readiness = crate::application::readiness::run_checks(&root);
        self.refresh_dependency_domains();
        self.normalize_readiness_selection();
    }
    fn refresh_dependency_domains(&mut self) {
        let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        self.dependency_domains = evaluate_project_dependency_domains(&root);
    }

    pub(super) fn readiness_actions(&self) -> Vec<ReadinessAction> {
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

    pub(super) fn execute_readiness_action(
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
                self.focus = PanelFocus::Input;
                self.logs
                    .push("readiness action: scope wizard started".to_string());
            }
            ReadinessAction::CreatePersona => {
                self.wizard = Some(CreationWizard::new_persona());
                self.focus = PanelFocus::Input;
                self.logs
                    .push("readiness action: persona wizard started".to_string());
            }
            ReadinessAction::CreateSkill => {
                self.wizard = Some(CreationWizard::new_skill());
                self.focus = PanelFocus::Input;
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
                    RuntimeEvent::MaestroNarration {
                        agent_name,
                        phase,
                        detail,
                    } => format!("🎼 {} [{}]: {}", agent_name, phase, detail),
                    RuntimeEvent::MaestroHeartbeat {
                        agent_name,
                        elapsed_secs,
                    } => format!("💓 {} running ({}s elapsed)", agent_name, elapsed_secs),
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

    pub(super) fn handle_key_event(&mut self, key: KeyEvent) -> Option<UserAction> {
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

        // Architect Mode: directives picker (select stage)
        if self.mode == UIMode::Architect {
            match key.code {
                KeyCode::Up => {
                    if let Some(picker) = &mut self.architect_picker {
                        picker.move_up();
                    }
                    return None;
                }
                KeyCode::Down => {
                    if let Some(picker) = &mut self.architect_picker {
                        picker.move_down();
                    }
                    return None;
                }
                KeyCode::Esc => {
                    self.return_to_workspace();
                    self.logs.push("🛰️ Workspace runtime monitor".to_string());
                    return None;
                }
                KeyCode::Enter => {
                    let selected = self
                        .architect_picker
                        .as_ref()
                        .and_then(|picker| picker.selected())
                        .map(|entry| {
                            (
                                entry.file_name.clone(),
                                entry.read_only,
                                entry.label.clone(),
                            )
                        });
                    return match selected {
                        None => {
                            self.logs.push(
                                "Directives editor: no directive available to edit.".to_string(),
                            );
                            None
                        }
                        Some((_, true, label)) => {
                            self.logs.push(format!(
                                "🔒 {label} is the immutable Maestro directive and cannot be edited or archived."
                            ));
                            None
                        }
                        Some((file_name, false, label)) => {
                            match self.architect_selection_target() {
                                Some(target) => {
                                    self.logs.push(format!(
                                        "✏️ Editing {} directive: {label}",
                                        target.kind_label()
                                    ));
                                    Some(UserAction::StartDirectiveAuthoring {
                                    target,
                                    operation:
                                        crate::application::interview_bot::DirectiveOperation::Edit,
                                    file_name,
                                })
                                }
                                None => None,
                            }
                        }
                    };
                }
                _ => return None,
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
                } else if command == "/architect" || command == "/core" {
                    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
                    let governance = MarkdownGovernance::new(root);
                    self.enter_directive_select(&governance);
                    None
                } else if command == "/monitor" {
                    self.return_to_workspace();
                    self.logs.push("🛰️ Workspace runtime monitor".to_string());
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

    pub(super) fn current_input_title(&self) -> String {
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
    pub(super) fn apply_wizard_submission(
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
