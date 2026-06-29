use super::*;

pub(super) async fn enqueue_interview_question(
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
    let maestro_online = session.maestro_online;
    drop(session);

    app.maestro_message_id = Some(question_id);
    app.logs.push(format!("maestro: {}", question_text));

    // When a model is online (Option B), the live Maestro role is the single
    // voice on the environment bus; do not also emit the scripted question to
    // avoid a duplicate Maestro voice in shared history.
    if !maestro_online {
        if let Some(env) = environment {
            let _ = env
                .publish(Message::new(
                    "Maestro".to_string(),
                    format!("Interview question {}: {}", next_turn, question_text),
                    None,
                ))
                .await;
        }
    }

    Ok(true)
}

pub(super) async fn enqueue_directive_question(
    app: &mut TuiApp,
    environment: Option<&Arc<Environment>>,
) -> Result<bool> {
    let Some(bot) = app.interview_bot.clone() else {
        return Ok(false);
    };
    let Some(session_lock) = app.interview_session.clone() else {
        return Ok(false);
    };

    let mut session = session_lock.write().await;
    let questions: Vec<&'static str> = session
        .target
        .as_ref()
        .map(|target| target.authoring_questions())
        .unwrap_or_default();
    let index = session.turn_count as usize;

    if index >= questions.len() {
        match bot.build_directive_proposal(&session) {
            Ok(proposal) => {
                session.proposed_changes = Some(proposal);
                session.approval_pending = true;
                drop(session);
                app.approval_modal_visible = true;
                app.logs
                    .push("🧾 Draft ready. Approve (y) or reject (n).".to_string());
            }
            Err(error) => {
                drop(session);
                app.logs
                    .push(format!("❌ Could not build directive draft: {error}"));
            }
        }
        return Ok(false);
    }

    let question_text = questions[index].to_string();
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
            .publish(Message::new("Maestro".to_string(), question_text, None))
            .await;
    }

    Ok(true)
}

/// Persist the single directive draft produced by interview-driven authoring.
///
/// Personas and skills are validated and overwritten through governance; an
/// edited scope keeps its existing sequence number and is overwritten directly
/// (scope sequence validation only admits next-in-sequence numbers for new
/// scopes).
pub(super) async fn apply_directive_proposal(
    app: &mut TuiApp,
    governance: &MarkdownGovernance,
) -> Result<PathBuf> {
    let session_lock = app
        .interview_session
        .clone()
        .ok_or_else(|| anyhow::anyhow!("no active interview session"))?;

    let (target, proposal) = {
        let session = session_lock.read().await;
        (session.target.clone(), session.proposed_changes.clone())
    };
    let target = target.ok_or_else(|| anyhow::anyhow!("no directive target"))?;
    let proposal = proposal.ok_or_else(|| anyhow::anyhow!("no proposal to apply"))?;

    let path = match &target {
        crate::application::interview_bot::DirectiveTarget::Persona { .. } => {
            let (file_name, content) = proposal
                .persona_drafts
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("empty persona draft"))?;
            persist_submission(governance, WizardSubmission::Persona { file_name, content })?
        }
        crate::application::interview_bot::DirectiveTarget::Skill { persona, .. } => {
            let (file_name, content) = proposal
                .skill_drafts
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("empty skill draft"))?;
            persist_submission(
                governance,
                WizardSubmission::Skill {
                    persona_name: persona.clone(),
                    file_name,
                    content,
                },
            )?
        }
        crate::application::interview_bot::DirectiveTarget::Scope { .. } => {
            let (file_name, content) = proposal
                .scope_drafts
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("empty scope draft"))?;
            governance.ensure_directories()?;
            let path = governance.scopes_dir().join(&file_name);
            std::fs::write(&path, content)?;
            path
        }
    };

    // AC10 hand-off: after the Project Manager writes a scope, Maestro reads it,
    // derives the additions each non-Maestro persona needs, audits dependencies,
    // and surfaces the required next actions in the Workspace monitor.
    if matches!(
        &target,
        crate::application::interview_bot::DirectiveTarget::Scope { .. }
    ) {
        if let Some(bot) = app.interview_bot.clone() {
            let session_snapshot = { session_lock.read().await.clone() };
            let project_deps = crate::application::project_deps::ProjectDepsConfig::load(None).ok();
            match bot.author_scope_with_additions(&session_snapshot, project_deps.as_ref()) {
                Ok(plan) => {
                    app.logs.push(format!(
                        "🧭 Maestro hand-off — {} next action(s) for the Workspace monitor:",
                        plan.next_actions.len()
                    ));
                    for action in plan.next_actions {
                        app.logs.push(format!("  → {action}"));
                    }
                }
                Err(error) => {
                    app.logs
                        .push(format!("⚠️ Maestro hand-off skipped: {error}"));
                }
            }
        }
    }

    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    app.readiness = crate::application::readiness::run_checks(&root);
    Ok(path)
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

pub(super) async fn apply_interview_scope_proposals(
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

pub(super) async fn run_maestro_wakeup_check(
    app: &mut TuiApp,
    environment: Option<&Arc<Environment>>,
    runtime: Option<&Arc<AgentRuntime>>,
) -> bool {
    let Some(env) = environment else {
        app.logs.push(
            "⚠️ Maestro runtime is not connected. Configure provider and model in maestro/config.yml."
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
        "⚠️ Maestro did not answer wake-up check. Configure provider/model in maestro/config.yml and restart interview."
            .to_string(),
    );
    app.maestro_message_id = None;
    false
}

/// Scan the latest unscanned `Maestro` bus message for a governed-file proposal
/// and, when found, stage it for confirmation (LLM-driven engine only).
///
/// Returns `true` when changes were staged. Conversational messages, already
/// scanned messages, and changes targeting the immutable Maestro persona never
/// stage anything. A parse failure is treated as a normal question: the message
/// id is recorded so it is not re-parsed on subsequent ticks.
pub(super) async fn detect_and_stage_maestro_proposal(
    app: &mut TuiApp,
    history: &[Message],
) -> bool {
    let Some(session_lock) = app.interview_session.clone() else {
        return false;
    };

    {
        let session = session_lock.read().await;
        if !session.engine.is_llm_driven() || session.confirmation_pending {
            return false;
        }
    }

    let Some(latest) = history
        .iter()
        .rev()
        .find(|msg| msg.sender().eq_ignore_ascii_case("maestro"))
    else {
        return false;
    };
    let latest_id = latest.id();

    {
        let session = session_lock.read().await;
        if session.last_parsed_maestro_msg == Some(latest_id) {
            return false;
        }
    }

    let parsed = crate::application::interview_bot::parse_directive_proposals(latest.content());

    let mut session = session_lock.write().await;
    session.last_parsed_maestro_msg = Some(latest_id);

    let changes = match parsed {
        Ok(changes) => changes,
        Err(_) => return false,
    };

    let staged: Vec<_> = changes
        .into_iter()
        .filter(|change| !change.target.targets_maestro())
        .collect();

    if staged.is_empty() {
        return false;
    }

    let count = staged.len();
    session.pending_changes = staged;
    session.confirmation_pending = true;
    drop(session);

    app.approval_modal_visible = true;
    app.logs.push(format!(
        "🧩 Maestro proposed {} file change(s). Approve (y) to write, or refine (n).",
        count
    ));
    true
}

/// Apply staged governed-file changes through markdown governance, logging each
/// outcome. Returns the number of files written (created/edited/updated).
pub(super) fn apply_directive_changes(
    app: &mut TuiApp,
    governance: &MarkdownGovernance,
    changes: &[crate::application::interview_bot::DirectiveFileChange],
) -> usize {
    use crate::application::interview_bot::{apply_directive_change, AppliedChange};

    let mut written = 0_usize;
    for change in changes {
        let kind = change.target.kind_label();
        match apply_directive_change(governance, change) {
            Ok(AppliedChange::Written(path)) => {
                written = written.saturating_add(1);
                app.logs
                    .push(format!("✅ {} written: {}", kind, path.display()));
            }
            Ok(AppliedChange::Archived(path)) => {
                app.logs
                    .push(format!("🗄️ {} archived: {}", kind, path.display()));
            }
            Ok(AppliedChange::Read { path, .. }) => {
                app.logs
                    .push(format!("📖 {} read: {}", kind, path.display()));
            }
            Err(error) => {
                app.logs
                    .push(format!("❌ Could not apply {} change: {}", kind, error));
            }
        }
    }
    written
}

/// Hand the authored project off to the Workspace runtime: install the full
/// governed sequential pipeline (replacing the interview's single Maestro
/// subscriber) and switch the UI to Workspace mode so the user's next
/// instruction orchestrates the whole agent team.
pub(super) async fn handoff_to_workspace(
    app: &mut TuiApp,
    governance: &MarkdownGovernance,
    runtime: Option<&Arc<AgentRuntime>>,
    router: Option<&crate::application::model_router::ModelRouter>,
    environment: Option<&Arc<Environment>>,
) {
    app.mode = UIMode::Workspace;
    app.approval_modal_visible = false;

    if let Some(session_lock) = &app.interview_session {
        let mut session = session_lock.write().await;
        session.pending_changes.clear();
        session.confirmation_pending = false;
        session.approval_pending = false;
    }

    match (runtime, router) {
        (Some(rt), Some(router)) => {
            let registrations =
                crate::application::persona_operations::registrations_from_governance(
                    router, governance,
                );
            let _ = rt.stop_all().await;
            rt.set_sequential_pipeline(registrations).await;
            app.logs
                .push("🛠️ Workspace ready — full agent team is online.".to_string());
        }
        _ => {
            app.logs.push(
                "🛠️ Workspace ready. Configure provider/model to orchestrate the full team."
                    .to_string(),
            );
        }
    }

    if let Some(env) = environment {
        let _ = env
            .publish(Message::new(
                "Maestro".to_string(),
                "✅ Project scaffolding is ready. Send your first build instruction and I'll coordinate the team."
                    .to_string(),
                None,
            ))
            .await;
    }
}
