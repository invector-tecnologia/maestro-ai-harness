use super::*;

pub fn render(frame: &mut Frame<'_>, app: &TuiApp) {
    let area = frame.area();

    // Architect Mode: directives picker (select stage)
    if app.mode == UIMode::Architect {
        let select_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(8), // Logo
                Constraint::Min(0),    // Directives picker
                Constraint::Length(5), // Command input
            ])
            .split(area);

        render_logo_panel(frame, select_rows[0]);
        render_architect_panel(frame, select_rows[1], app);
        render_input_panel(frame, select_rows[2], app);
        return;
    }

    // Interview author stage has special layout
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

fn render_architect_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect, app: &TuiApp) {
    let mut lines = vec![
        "Architect Mode — Project Directives".to_string(),
        "Up/Down navigate · Enter select · Esc back to monitor".to_string(),
        String::new(),
    ];

    match &app.architect_picker {
        Some(picker) if !picker.entries.is_empty() => {
            let mut current_group: Option<DirectiveGroup> = None;
            for (index, entry) in picker.entries.iter().enumerate() {
                if current_group != Some(entry.group) {
                    if current_group.is_some() {
                        lines.push(String::new());
                    }
                    lines.push(format!("[{}]", entry.group.title()));
                    current_group = Some(entry.group);
                }

                let pointer = if index == picker.cursor { ">" } else { " " };
                let lock = if entry.read_only {
                    " 🔒 read-only"
                } else {
                    ""
                };
                lines.push(format!("{pointer} {}{lock}", entry.label));
            }
        }
        _ => {
            lines
                .push("No directives yet. Use /new persona|scope|skill to author one.".to_string());
        }
    }

    let paragraph = Paragraph::new(lines.join("\n"))
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title("Architect Mode")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        );

    frame.render_widget(paragraph, area);
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
                .title("④ Readiness")
                .borders(Borders::ALL)
                .border_style(if app.focus == PanelFocus::Readiness {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else if all_ready {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                }),
        );

    frame.render_widget(paragraph.style(headline_style), chunks[0]);

    let mut actions_lines = vec![];

    actions_lines.push("Flow: ①Input → ②Orchestration → ③Agent Activity → ④Readiness".to_string());
    actions_lines.push(format!("Focus: {} (Tab cycles)", app.focus.role_label()));
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

pub(super) fn evaluate_project_dependency_domains(
    root: &std::path::Path,
) -> DependencyDomainsState {
    let deps_path = root.join("maestro").join("project-deps.yml");
    if !deps_path.exists() {
        return DependencyDomainsState {
            project_manifest_found: false,
            project_manifest_valid: false,
            project_required_checks_passed: false,
            project_failed_required: Vec::new(),
            project_failed_required_hints: Vec::new(),
            project_error: Some("maestro/project-deps.yml not found".to_string()),
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
            .title("③ Agent Activity")
            .borders(Borders::ALL)
            .border_style(panel_border_style(app.focus == PanelFocus::AgentActivity)),
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
            .title("② Orchestration")
            .borders(Borders::ALL)
            .border_style(panel_border_style(app.focus == PanelFocus::Orchestration)),
    );
    frame.render_widget(list, area);
}

fn render_input_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect, app: &TuiApp) {
    let is_focused = app.wizard.is_some() || app.focus == PanelFocus::Input;
    let paragraph = Paragraph::new(app.input.as_str())
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title(format!("① Input · {}", app.current_input_title()))
                .borders(Borders::ALL)
                .border_style(panel_border_style(is_focused)),
        );

    frame.render_widget(paragraph, area);

    if is_focused {
        let max_x = area.x + area.width.saturating_sub(2);
        let cursor_x = (area.x + 1 + app.input.chars().count() as u16).min(max_x);
        let cursor_y = area.y + 1;

        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

/// Number of seconds Maestro can spend in `think` before the interview panel
/// adds a hint that the model may be slow or unreachable.
const SLOW_THINK_HINT_SECS: u64 = 20;

/// Build the Maestro interview status line(s) from the live agent state.
///
/// Driven by the real `Maestro` agent health (not a static label) so the panel
/// reflects whether Maestro is genuinely thinking, idle/listening, or errored,
/// and surfaces a slow-model hint when a single `think` runs unusually long.
fn maestro_status_lines(
    approval_pending: bool,
    maestro_status: Option<&str>,
    maestro_online: bool,
    thinking_secs: Option<u64>,
) -> Vec<String> {
    let mut lines = Vec::new();
    if approval_pending {
        lines.push("  🔔 Awaiting your decision...".to_string());
    } else if maestro_status == Some("error") {
        lines.push("  ❌ Maestro hit an error — see the Orchestration log.".to_string());
    } else if maestro_status == Some("think") {
        let secs = thinking_secs.unwrap_or(0);
        lines.push(format!("  🧠 Thinking with Maestro… ({}s)", secs));
        if secs >= SLOW_THINK_HINT_SECS {
            lines.push(
                "  ⏳ The model is slow — check provider/model and timeout_ms in maestro/config.yml."
                    .to_string(),
            );
        }
    } else {
        let _ = maestro_online;
        lines.push("  🎧 Listening…".to_string());
    }
    lines
}

fn render_maestro_panel(frame: &mut Frame<'_>, area: ratatui::layout::Rect, app: &TuiApp) {
    let mut lines = vec![];

    if let Some(session_lock) = &app.interview_session {
        let (turn, engine, maestro_online) = session_lock
            .try_read()
            .ok()
            .map(|session| (session.turn_count, session.engine, session.maestro_online))
            .unwrap_or((
                0,
                crate::application::interview_bot::InterviewEngine::GuidedSetup,
                false,
            ));
        lines.push("🤖 Maestro Interview".to_string());
        lines.push(format!(
            "  Engine: {} ({})",
            engine.label(),
            if maestro_online {
                "model online"
            } else {
                "model offline"
            }
        ));
        lines.push(format!("  Turn: {}/10", turn));
        let maestro_status = app
            .agents
            .iter()
            .find(|agent| agent.name == "Maestro")
            .map(|agent| agent.status.clone());
        let thinking_secs = app.thinking_since.map(|since| since.elapsed().as_secs());
        lines.extend(maestro_status_lines(
            app.approval_modal_visible,
            maestro_status.as_deref(),
            maestro_online,
            thinking_secs,
        ));
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
            if !session.pending_changes.is_empty() {
                proposal_text.push("Maestro will write these governed files:".to_string());
                proposal_text.push("".to_string());
                for change in session.pending_changes.iter().take(6) {
                    proposal_text.push(format!(
                        "  • {} {} → {}",
                        change.op.label(),
                        change.target.kind_label(),
                        change.file_name
                    ));
                }
            } else if let Some(proposals) = &session.proposed_changes {
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

#[cfg(test)]
mod maestro_status_tests {
    use super::{maestro_status_lines, SLOW_THINK_HINT_SECS};

    #[test]
    fn shows_live_elapsed_while_thinking() {
        let lines = maestro_status_lines(false, Some("think"), true, Some(3));
        assert_eq!(lines.len(), 1, "fast think shows only the thinking line");
        assert!(
            lines[0].contains("Thinking with Maestro") && lines[0].contains("(3s)"),
            "expected live elapsed thinking line, got {:?}",
            lines
        );
    }

    #[test]
    fn appends_slow_hint_past_threshold() {
        let lines = maestro_status_lines(false, Some("think"), true, Some(SLOW_THINK_HINT_SECS));
        assert_eq!(lines.len(), 2, "slow think adds a hint line");
        assert!(
            lines[1].contains("model is slow") && lines[1].contains("timeout_ms"),
            "expected slow-model hint, got {:?}",
            lines
        );
    }

    #[test]
    fn listens_when_idle() {
        let lines = maestro_status_lines(false, Some("idle"), true, None);
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0].contains("Listening"),
            "idle Maestro listens, got {:?}",
            lines
        );
    }

    #[test]
    fn surfaces_error_state() {
        let lines = maestro_status_lines(false, Some("error"), true, None);
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0].contains("error"),
            "error state is surfaced, got {:?}",
            lines
        );
    }

    #[test]
    fn approval_takes_precedence_over_thinking() {
        let lines = maestro_status_lines(true, Some("think"), true, Some(99));
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0].contains("Awaiting your decision"),
            "approval prompt wins, got {:?}",
            lines
        );
    }
}
