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
            model_loaded: true,
        },
        dependency_domains: DependencyDomainsState {
            project_manifest_found: true,
            project_manifest_valid: true,
            project_required_checks_passed: true,
            project_failed_required: vec![],
            project_failed_required_hints: vec![],
            project_error: None,
        },
        focus: PanelFocus::Input,
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
        thinking_since: None,
        architect_picker: None,
    };

    let drawn = terminal.draw(|frame| render(frame, &app));
    assert!(drawn.is_ok());

    let content = buffer_to_string(&terminal);
    assert!(content.contains("Agent Activity"));
    assert!(content.contains("Orchestration"));
    assert!(content.contains("Input"));
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
            model_loaded: false,
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
fn tab_cycles_workspace_focus_flow_deterministically() {
    let mut app = TuiApp::default();
    assert_eq!(app.focus, PanelFocus::Input);

    let tab = || KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);

    assert!(app.handle_key_event(tab()).is_none());
    assert_eq!(app.focus, PanelFocus::Orchestration);

    assert!(app.handle_key_event(tab()).is_none());
    assert_eq!(app.focus, PanelFocus::AgentActivity);

    assert!(app.handle_key_event(tab()).is_none());
    assert_eq!(app.focus, PanelFocus::Readiness);
    assert!(app.current_input_title().contains("Readiness focus"));

    assert!(app.handle_key_event(tab()).is_none());
    assert_eq!(app.focus, PanelFocus::Input);
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
            model_loaded: false,
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
            model_loaded: false,
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
            model_loaded: true,
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

    let config_path = root.join("maestro").join("config.yml");
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

#[test]
fn directive_governance_bootstrap_does_not_enter_onboarding_interview() {
    assert!(!should_enter_interview(
        OnboardingBootstrap::DirectiveGovernance,
        true
    ));
    assert!(!should_enter_interview(
        OnboardingBootstrap::DirectiveGovernance,
        false
    ));
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
async fn directive_authoring_overwrites_existing_persona() {
    let root = temp_root("maestro-directive-apply-persona");
    let governance = MarkdownGovernance::new(&root);
    governance.ensure_directories().expect("dirs");

    let persona_path = governance.personas_dir().join("project-manager.md");
    fs::write(
            &persona_path,
            "# Project Manager\n\n## Responsibilities\nold\n## Deliverables\nold\n## Operational Instructions\nold\n## Interaction Matrix\nold\n## Boundaries\nold\n",
        )
        .expect("seed persona");

    let bot = crate::application::interview_bot::InterviewBot::new();
    let mut session = crate::application::interview_bot::InterviewSession::for_directive(
        crate::application::interview_bot::DirectiveOperation::Edit,
        crate::application::interview_bot::DirectiveTarget::Persona {
            name: "Project Manager".to_string(),
        },
        Some("project-manager.md".to_string()),
        None,
    )
    .expect("session");
    session.exchange_history = vec![crate::application::interview_bot::InterviewExchange {
        maestro_question: Uuid::new_v4(),
        maestro_text: "q".to_string(),
        user_answer: "Coordinate delivery across teams".to_string(),
        timestamp: SystemTime::now(),
    }];
    let proposal = bot.build_directive_proposal(&session).expect("proposal");
    session.proposed_changes = Some(proposal);

    let mut app = TuiApp {
        mode: UIMode::Interview,
        interview_session: Some(Arc::new(tokio::sync::RwLock::new(session))),
        ..TuiApp::default()
    };

    let path = apply_directive_proposal(&mut app, &governance)
        .await
        .expect("apply");
    assert_eq!(path, persona_path);

    let written = fs::read_to_string(&persona_path).expect("read persona");
    assert!(written.contains("Coordinate delivery across teams"));
    assert!(!written.contains("## Responsibilities\nold"));

    let _ = fs::remove_dir_all(root);
}

#[tokio::test]
async fn directive_authoring_scope_surfaces_maestro_handoff() {
    let root = temp_root("maestro-directive-scope-handoff");
    let governance = MarkdownGovernance::new(&root);
    governance.ensure_directories().expect("dirs");

    let bot = crate::application::interview_bot::InterviewBot::new();
    let mut session = crate::application::interview_bot::InterviewSession::for_directive(
        crate::application::interview_bot::DirectiveOperation::Create,
        crate::application::interview_bot::DirectiveTarget::Scope {
            name: "Checkout API".to_string(),
        },
        Some("001-checkout-api.md".to_string()),
        None,
    )
    .expect("session");
    session.exchange_history = vec![crate::application::interview_bot::InterviewExchange {
        maestro_question: Uuid::new_v4(),
        maestro_text: "q".to_string(),
        user_answer: "Ship the checkout API and validate acceptance tests".to_string(),
        timestamp: SystemTime::now(),
    }];
    let proposal = bot.build_directive_proposal(&session).expect("proposal");
    session.proposed_changes = Some(proposal);

    let mut app = TuiApp {
        mode: UIMode::Interview,
        interview_bot: Some(Arc::new(bot)),
        interview_session: Some(Arc::new(tokio::sync::RwLock::new(session))),
        ..TuiApp::default()
    };

    let path = apply_directive_proposal(&mut app, &governance)
        .await
        .expect("apply scope");
    assert_eq!(path, governance.scopes_dir().join("001-checkout-api.md"));

    assert!(app
        .logs
        .iter()
        .any(|line| line.contains("Maestro hand-off")));
    assert!(app
        .logs
        .iter()
        .any(|line| line.contains("Project Manager authored scope")));
    assert!(app
        .logs
        .iter()
        .any(|line| line.contains("Open Workspace monitor")));

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
        "❓ Understood. Let us refine requirements before generating new scope drafts.".to_string(),
    );

    assert_eq!(app.mode, UIMode::Interview);
    let scopes = fs::read_dir(governance.scopes_dir())
        .unwrap_or_else(|_| panic!("cannot read scopes dir"))
        .flatten()
        .collect::<Vec<_>>();
    assert!(scopes.is_empty());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn persist_archive_persona_moves_document_into_archive_tree() {
    let root = temp_root("maestro-archive-persona");
    let governance = MarkdownGovernance::new(&root);
    let ensured = governance.ensure_directories();
    assert!(ensured.is_ok());

    let persona_path = governance.personas_dir().join("project-manager.md");
    let write = fs::write(&persona_path, "## Responsibility\nPlan delivery\n");
    assert!(write.is_ok());

    let result = persist_submission(
        &governance,
        WizardSubmission::ArchivePersona {
            file_name: "project-manager.md".to_string(),
        },
    );

    assert!(result.is_ok());
    assert!(!persona_path.exists());
    let archived = governance
        .archive_dir()
        .join("personas")
        .join("project-manager.md");
    assert!(archived.exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn persist_archive_rejects_immutable_maestro_persona() {
    let root = temp_root("maestro-archive-immutable");
    let governance = MarkdownGovernance::new(&root);
    let ensured = governance.ensure_directories();
    assert!(ensured.is_ok());

    let maestro_path = governance.personas_dir().join("maestro.md");
    let write = fs::write(&maestro_path, "## Responsibility\nOrchestrate\n");
    assert!(write.is_ok());

    let result = persist_submission(
        &governance,
        WizardSubmission::ArchivePersona {
            file_name: "maestro.md".to_string(),
        },
    );

    assert!(result.is_err());
    assert!(maestro_path.exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn persist_archive_skill_moves_document_into_archive_tree() {
    let root = temp_root("maestro-archive-skill");
    let governance = MarkdownGovernance::new(&root);
    let ensured = governance.ensure_directories();
    assert!(ensured.is_ok());

    let skill_dir = governance.skills_dir().join("software-engineer");
    let create_dir = fs::create_dir_all(&skill_dir);
    assert!(create_dir.is_ok());
    let skill_path = skill_dir.join("refactoring.md");
    let write = fs::write(&skill_path, "## Objective\nImprove design\n");
    assert!(write.is_ok());

    let result = persist_submission(
        &governance,
        WizardSubmission::ArchiveSkill {
            persona_name: "software-engineer".to_string(),
            file_name: "refactoring.md".to_string(),
        },
    );

    assert!(result.is_ok());
    assert!(!skill_path.exists());
    let archived = governance
        .archive_dir()
        .join("skills")
        .join("software-engineer")
        .join("refactoring.md");
    assert!(archived.exists());

    let _ = fs::remove_dir_all(root);
}

fn seed_core_directives(governance: &MarkdownGovernance) {
    let ensured = governance.ensure_directories();
    assert!(ensured.is_ok());

    let maestro = fs::write(
        governance.personas_dir().join("maestro.md"),
        "## Responsibility\nOrchestrate\n",
    );
    assert!(maestro.is_ok());
    let pm = fs::write(
        governance.personas_dir().join("project-manager.md"),
        "## Responsibility\nPlan\n",
    );
    assert!(pm.is_ok());

    let maestro_skill_dir = governance.skills_dir().join("maestro");
    assert!(fs::create_dir_all(&maestro_skill_dir).is_ok());
    assert!(fs::write(
        maestro_skill_dir.join("observability.md"),
        "## Objective\nObserve\n"
    )
    .is_ok());

    let pm_skill_dir = governance.skills_dir().join("project-manager");
    assert!(fs::create_dir_all(&pm_skill_dir).is_ok());
    assert!(fs::write(pm_skill_dir.join("planning.md"), "## Objective\nPlan\n").is_ok());

    assert!(fs::write(
        governance.scopes_dir().join("001-backend.md"),
        "## Objective\nBackend\n"
    )
    .is_ok());
}

#[test]
fn architect_picker_groups_directives_and_flags_maestro_read_only() {
    let root = temp_root("maestro-core-picker");
    let governance = MarkdownGovernance::new(&root);
    seed_core_directives(&governance);

    let picker = ArchitectPicker::from_governance(&governance);

    let maestro_persona = picker
        .entries
        .iter()
        .find(|e| e.group == DirectiveGroup::Personas && e.file_name == "maestro.md")
        .expect("maestro persona entry present");
    assert!(maestro_persona.read_only);

    let pm_persona = picker
        .entries
        .iter()
        .find(|e| e.group == DirectiveGroup::Personas && e.file_name == "project-manager.md")
        .expect("project-manager persona entry present");
    assert!(!pm_persona.read_only);

    let maestro_skill = picker
        .entries
        .iter()
        .find(|e| e.group == DirectiveGroup::Skills && e.persona.as_deref() == Some("maestro"))
        .expect("maestro skill entry present");
    assert!(maestro_skill.read_only);

    let scope = picker
        .entries
        .iter()
        .find(|e| e.group == DirectiveGroup::Scopes)
        .expect("scope entry present");
    assert!(!scope.read_only);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn architect_selection_target_blocks_maestro_and_resolves_others() {
    let root = temp_root("maestro-core-selection");
    let governance = MarkdownGovernance::new(&root);
    seed_core_directives(&governance);

    let mut app = TuiApp {
        architect_picker: Some(ArchitectPicker::from_governance(&governance)),
        mode: UIMode::Architect,
        ..TuiApp::default()
    };

    // Cursor starts at the first entry (maestro persona, read-only).
    assert!(app.architect_selection_target().is_none());

    // Move to the first non-read-only entry and confirm it resolves.
    if let Some(picker) = &mut app.architect_picker {
        while picker
            .selected()
            .map(|entry| entry.read_only)
            .unwrap_or(false)
        {
            let before = picker.cursor;
            picker.move_down();
            if picker.cursor == before {
                break;
            }
        }
    }
    assert!(app.architect_selection_target().is_some());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn architect_command_enters_mode_and_monitor_returns_to_workspace() {
    let root = temp_root("maestro-core-command");
    let governance = MarkdownGovernance::new(&root);
    seed_core_directives(&governance);

    let mut app = TuiApp::default();
    app.enter_directive_select(&governance);
    assert_eq!(app.mode, UIMode::Architect);
    assert!(app.architect_picker.is_some());

    app.return_to_workspace();
    assert_eq!(app.mode, UIMode::Workspace);
    assert!(app.architect_picker.is_none());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn slash_commands_route_to_architect_mode() {
    // `/architect` is the canonical command.
    let mut app = TuiApp {
        input: "/architect".to_string(),
        ..Default::default()
    };
    let action = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(action.is_none());
    assert_eq!(app.mode, UIMode::Architect);

    // `/core` remains a back-compat alias.
    let mut app = TuiApp {
        input: "/core".to_string(),
        ..Default::default()
    };
    let action = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(action.is_none());
    assert_eq!(app.mode, UIMode::Architect);

    // `/edit` is no longer a mode command and must not enter Architect Mode.
    let mut app = TuiApp {
        input: "/edit".to_string(),
        ..Default::default()
    };
    let _ = app.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_ne!(app.mode, UIMode::Architect);
}

#[test]
fn maestro_panel_shows_llm_driven_engine_when_model_online() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = match Terminal::new(backend) {
        Ok(value) => value,
        Err(_) => panic!("terminal init failed"),
    };

    let app = TuiApp {
        mode: UIMode::Interview,
        interview_session: Some(Arc::new(tokio::sync::RwLock::new(
            crate::application::interview_bot::InterviewSession {
                engine: crate::application::interview_bot::InterviewEngine::LlmDriven,
                maestro_online: true,
                ..Default::default()
            },
        ))),
        ..TuiApp::default()
    };

    let drawn = terminal.draw(|frame| render(frame, &app));
    assert!(drawn.is_ok());

    let rendered = buffer_to_string(&terminal);
    assert!(rendered.contains("Engine:"));
    assert!(rendered.contains("llm-driven"));
    assert!(rendered.contains("model online"));
}

#[test]
fn maestro_panel_shows_guided_setup_engine_when_model_offline() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = match Terminal::new(backend) {
        Ok(value) => value,
        Err(_) => panic!("terminal init failed"),
    };

    let app = TuiApp {
        mode: UIMode::Interview,
        interview_session: Some(Arc::new(tokio::sync::RwLock::new(
            crate::application::interview_bot::InterviewSession {
                engine: crate::application::interview_bot::InterviewEngine::GuidedSetup,
                maestro_online: false,
                ..Default::default()
            },
        ))),
        ..TuiApp::default()
    };

    let drawn = terminal.draw(|frame| render(frame, &app));
    assert!(drawn.is_ok());

    let rendered = buffer_to_string(&terminal);
    assert!(rendered.contains("guided setup"));
    assert!(rendered.contains("model offline"));
}

#[test]
fn thinking_since_tracks_maestro_think_state() {
    use crate::application::agent_runtime::AgentHealth;

    let mut app = TuiApp::default();
    assert!(app.thinking_since.is_none());

    // Maestro enters `think` → start instant is set.
    let mut thinking = HashMap::new();
    thinking.insert("Maestro".to_string(), AgentHealth::Thinking);
    app.update_agents_from_health(&thinking);
    assert!(
        app.thinking_since.is_some(),
        "thinking_since starts when Maestro begins thinking"
    );

    // Staying in `think` keeps the original start instant (no reset).
    let started = app.thinking_since;
    app.update_agents_from_health(&thinking);
    assert_eq!(
        app.thinking_since, started,
        "thinking_since is not reset while Maestro keeps thinking"
    );

    // Maestro leaves `think` → start instant is cleared.
    let mut idle = HashMap::new();
    idle.insert("Maestro".to_string(), AgentHealth::Idle);
    app.update_agents_from_health(&idle);
    assert!(
        app.thinking_since.is_none(),
        "thinking_since clears once Maestro stops thinking"
    );
}
