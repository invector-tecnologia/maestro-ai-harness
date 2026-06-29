use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::SystemTime;
use thiserror::Error;
use uuid::Uuid;

use crate::application::persona_operations::PersonaRuntimeRole;
use crate::application::project_deps::ProjectDepsConfig;

/// Represents a single Q&A exchange in the interview
#[derive(Debug, Clone)]
pub struct InterviewExchange {
    pub maestro_question: Uuid,
    pub maestro_text: String,
    pub user_answer: String,
    pub timestamp: SystemTime,
}

/// Directive operation requested through Core Mode and authored via Interview Mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DirectiveOperation {
    #[default]
    Create,
    Edit,
    Update,
    Delete,
}

impl DirectiveOperation {
    /// Human-readable label used in prompts, logs, and error messages.
    pub fn label(&self) -> &'static str {
        match self {
            DirectiveOperation::Create => "create",
            DirectiveOperation::Edit => "edit",
            DirectiveOperation::Update => "update",
            DirectiveOperation::Delete => "delete",
        }
    }

    /// Whether the operation needs an existing directive loaded before authoring.
    pub fn requires_existing_target(&self) -> bool {
        matches!(
            self,
            DirectiveOperation::Edit | DirectiveOperation::Update | DirectiveOperation::Delete
        )
    }
}

/// Directive target a Core Mode operation acts upon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectiveTarget {
    Persona { name: String },
    Skill { persona: String, name: String },
    Scope { name: String },
}

impl DirectiveTarget {
    /// Directive kind label used for grouping and messaging.
    pub fn kind_label(&self) -> &'static str {
        match self {
            DirectiveTarget::Persona { .. } => "persona",
            DirectiveTarget::Skill { .. } => "skill",
            DirectiveTarget::Scope { .. } => "scope",
        }
    }

    /// Whether this target resolves to the immutable Maestro persona or its skills.
    pub fn targets_maestro(&self) -> bool {
        match self {
            DirectiveTarget::Persona { name } => is_maestro_identity(name),
            DirectiveTarget::Skill { persona, .. } => is_maestro_identity(persona),
            DirectiveTarget::Scope { .. } => false,
        }
    }

    /// Display name used in prompts, summaries, and logs.
    pub fn display_name(&self) -> &str {
        match self {
            DirectiveTarget::Persona { name } => name,
            DirectiveTarget::Skill { name, .. } => name,
            DirectiveTarget::Scope { name } => name,
        }
    }

    /// Targeted authoring questions for this directive kind.
    ///
    /// These replace the former form-based creation wizard fields so directive
    /// authoring has a single interview-driven path.
    pub fn authoring_questions(&self) -> Vec<&'static str> {
        match self {
            DirectiveTarget::Persona { .. } => vec![
                "What is this persona's primary purpose?",
                "What are this persona's core responsibilities?",
                "Which personas does it collaborate with, and how?",
                "What quality criteria must its work satisfy?",
            ],
            DirectiveTarget::Skill { .. } => vec![
                "What capability does this skill add to the persona?",
                "When should the persona apply this skill?",
                "What does a successful outcome look like?",
            ],
            DirectiveTarget::Scope { .. } => vec![
                "What is the objective of this scope?",
                "What work items does this scope include?",
                "What are the acceptance criteria?",
            ],
        }
    }

    /// Default markdown file name for a freshly created directive of this kind.
    fn default_file_name(&self) -> String {
        match self {
            DirectiveTarget::Persona { name } => format!("{}.md", slugify_persona_name(name)),
            DirectiveTarget::Skill { name, .. } => format!("{}.md", slugify_persona_name(name)),
            DirectiveTarget::Scope { name } => format!("{}.md", slugify_persona_name(name)),
        }
    }
}

fn is_maestro_identity(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.eq_ignore_ascii_case("maestro") || trimmed.eq_ignore_ascii_case("maestro.md")
}

/// Errors raised when validating a requested directive operation.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DirectiveOperationError {
    #[error("Maestro persona is immutable and cannot be the target of {operation} operations")]
    ImmutableMaestro { operation: &'static str },
    #[error("{operation} requires an existing target directive to be selected first")]
    MissingExistingTarget { operation: &'static str },
}

/// Validate a directive operation before any interview authoring begins.
///
/// Enforces Maestro immutability (no create/edit/update/delete may target Maestro)
/// and ensures mutation-of-existing operations carry a selected target file.
pub fn validate_directive_operation(
    operation: DirectiveOperation,
    target: &DirectiveTarget,
    target_file: Option<&str>,
) -> Result<(), DirectiveOperationError> {
    if target.targets_maestro() {
        return Err(DirectiveOperationError::ImmutableMaestro {
            operation: operation.label(),
        });
    }

    if operation.requires_existing_target() && target_file.unwrap_or("").trim().is_empty() {
        return Err(DirectiveOperationError::MissingExistingTarget {
            operation: operation.label(),
        });
    }

    Ok(())
}

/// Collected project needs extracted from interview responses
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersonaNeeds {
    pub project_name: String,
    pub project_type: String, // e.g., "SaaS", "CLI Tool", "Research Project"
    pub team_size: String,    // e.g., "Solo", "Small (3-5)", "Large (10+)"
    pub pain_points: Vec<String>,
    pub tech_stack: Vec<String>,
    pub recommended_personas: Vec<String>, // e.g., ["Project Manager", "Software Engineer"]
    pub recommended_skills: Vec<(String, String)>, // (persona_name, skill_name)
    pub recommended_scopes: Vec<String>,
    pub rag_domains: Vec<String>,
    pub kv_cache_optimization: bool,
}

/// Proposed changes to be reviewed by user before application
#[derive(Debug, Clone)]
pub struct ProposedChanges {
    pub persona_drafts: Vec<(String, String)>, // (file_name, markdown_content)
    pub skill_drafts: Vec<(String, String)>,
    pub scope_drafts: Vec<(String, String)>,
    pub summary: String, // Human-readable summary: "I recommend X personas..."
}

/// Non-Maestro personas eligible for scope-derived additions.
///
/// Maestro is intentionally excluded: it orchestrates authoring and is never a
/// derivation target for new skills.
pub const ADDITION_TARGET_PERSONAS: [&str; 4] = [
    "Project Manager",
    "Quality Assurance",
    "User Experience",
    "Software Engineer",
];

/// A persona skill addition Maestro derives by reading an authored scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonaAddition {
    pub persona: String,
    pub skill_name: String,
    pub rationale: String,
    pub file_name: String,
    pub content: String,
}

/// Outcome of the Project-Manager-first scope authoring pipeline.
///
/// The Project Manager writes the scope first (`scope_file_name` /
/// `scope_content`); Maestro then reads that written scope to derive the
/// `additions` each non-Maestro persona needs, and surfaces `next_actions` as a
/// hand-off to the Workspace monitor.
#[derive(Debug, Clone)]
pub struct ScopeAuthoringPlan {
    pub scope_file_name: String,
    pub scope_content: String,
    pub additions: Vec<PersonaAddition>,
    pub next_actions: Vec<String>,
}

/// Interview session state machine
#[derive(Debug, Clone)]
pub struct InterviewSession {
    pub exchange_history: Vec<InterviewExchange>,
    pub turn_count: u32,
    pub collected_needs: Option<PersonaNeeds>,
    pub proposed_changes: Option<ProposedChanges>,
    pub approval_pending: bool,
    pub session_start: SystemTime,
    pub operation: DirectiveOperation,
    pub target: Option<DirectiveTarget>,
    pub target_file: Option<String>,
    pub existing_content: Option<String>,
}

impl Default for InterviewSession {
    fn default() -> Self {
        Self {
            exchange_history: Vec::new(),
            turn_count: 0,
            collected_needs: None,
            proposed_changes: None,
            approval_pending: false,
            session_start: SystemTime::now(),
            operation: DirectiveOperation::Create,
            target: None,
            target_file: None,
            existing_content: None,
        }
    }
}

impl InterviewSession {
    /// Build a directive-scoped session for a validated Core Mode operation.
    pub fn for_directive(
        operation: DirectiveOperation,
        target: DirectiveTarget,
        target_file: Option<String>,
        existing_content: Option<String>,
    ) -> Result<Self, DirectiveOperationError> {
        validate_directive_operation(operation, &target, target_file.as_deref())?;

        Ok(Self {
            operation,
            target: Some(target),
            target_file,
            existing_content,
            ..Self::default()
        })
    }
}

/// Interview bot that conducts the setup interview
#[derive(Debug, Clone)]
pub struct InterviewBot {
    #[allow(dead_code)]
    question_templates: Vec<String>,
    #[allow(dead_code)]
    maestro_persona: Option<Arc<PersonaRuntimeRole>>,
}

impl InterviewBot {
    /// Create a new interview bot with predefined questions
    pub fn new() -> Self {
        let question_templates = vec![
            "What is the primary purpose or vision for your project?".to_string(),
            "How large is your team, and what are their primary roles or skill sets?".to_string(),
            "What technical challenges or pain points do you anticipate?".to_string(),
            "What technology stack or frameworks are you planning to use?".to_string(),
            "What does success look like for your first milestone?".to_string(),
            "Are there any specific architectural concerns or constraints we should keep in mind?"
                .to_string(),
            "How do you prefer to organize your project work (agile, waterfall, kanban)?"
                .to_string(),
            "What kind of documentation or knowledge management is important for your team?"
                .to_string(),
        ];

        Self {
            question_templates,
            maestro_persona: None,
        }
    }

    /// Get the question for the given turn (1-indexed)
    pub fn get_question(&self, turn: u32) -> Option<String> {
        if turn > 0 && turn <= self.question_templates.len() as u32 {
            Some(self.question_templates[(turn - 1) as usize].clone())
        } else {
            None
        }
    }

    /// Process a user answer and advance the interview
    pub async fn process_user_answer(
        &self,
        session: &mut InterviewSession,
        user_answer: String,
        maestro_question_msg_id: Uuid,
    ) -> Result<()> {
        let mut answer_attached = false;
        if let Some(exchange) = session.exchange_history.last_mut() {
            if exchange.user_answer.is_empty() {
                exchange.user_answer = user_answer.clone();
                exchange.timestamp = SystemTime::now();
                answer_attached = true;
            }
        }

        if !answer_attached {
            let fallback_question = self
                .get_question(session.turn_count.saturating_add(1))
                .unwrap_or_else(|| "Interview follow-up question".to_string());
            session.exchange_history.push(InterviewExchange {
                maestro_question: maestro_question_msg_id,
                maestro_text: fallback_question,
                user_answer,
                timestamp: SystemTime::now(),
            });
        }

        session.turn_count += 1;

        // After 7 turns, trigger analysis
        if session.turn_count >= 7 && session.proposed_changes.is_none() {
            let needs = self.analyze_conversation(session).await?;
            session.collected_needs = Some(needs.clone());

            // Generate proposals
            let proposals = self.generate_proposals(&needs)?;
            session.proposed_changes = Some(proposals);
            session.approval_pending = true;
        }

        Ok(())
    }

    /// Analyze conversation to extract PersonaNeeds (LLM-driven or heuristic)
    pub(crate) async fn analyze_conversation(
        &self,
        session: &InterviewSession,
    ) -> Result<PersonaNeeds> {
        let mut needs = PersonaNeeds::default();

        // Heuristic extraction from exchanges (simplified; in production, call Maestro's LLM)
        for exchange in &session.exchange_history {
            let answer_lower = exchange.user_answer.to_lowercase();

            // Extract project type signals
            if answer_lower.contains("saas") || answer_lower.contains("web") {
                needs.project_type = "SaaS/Web Application".to_string();
            }
            if answer_lower.contains("cli") || answer_lower.contains("command") {
                needs.project_type = "CLI Tool".to_string();
            }
            if answer_lower.contains("research") || answer_lower.contains("poc") {
                needs.project_type = "Research/POC".to_string();
            }

            // Extract team size signals
            if answer_lower.contains("solo") || answer_lower.contains("myself") {
                needs.team_size = "Solo Developer".to_string();
            } else if answer_lower.contains("3") || answer_lower.contains("5") {
                needs.team_size = "Small Team (3-5)".to_string();
            } else if answer_lower.contains("10") || answer_lower.contains("large") {
                needs.team_size = "Large Team (10+)".to_string();
            }

            // Extract tech stack signals
            if answer_lower.contains("rust") {
                needs.tech_stack.push("Rust".to_string());
            }
            if answer_lower.contains("python") {
                needs.tech_stack.push("Python".to_string());
            }
            if answer_lower.contains("typescript") || answer_lower.contains("javascript") {
                needs.tech_stack.push("TypeScript/JavaScript".to_string());
            }

            // Extract pain points
            if answer_lower.contains("performance") {
                needs
                    .pain_points
                    .push("Performance optimization".to_string());
            }
            if answer_lower.contains("scaling") || answer_lower.contains("scale") {
                needs.pain_points.push("Scalability".to_string());
            }
            if answer_lower.contains("maintainability") || answer_lower.contains("maintain") {
                needs.pain_points.push("Code maintainability".to_string());
            }
        }

        // Recommend personas based on extracted needs
        if !needs.tech_stack.is_empty() {
            needs
                .recommended_personas
                .push("Software Engineer".to_string());
        }
        if needs.project_type.contains("SaaS") {
            needs
                .recommended_personas
                .push("Project Manager".to_string());
        }
        if needs.pain_points.iter().any(|p| p.contains("Scalability")) {
            needs
                .recommended_personas
                .push("Quality Assurance".to_string());
        }
        if needs.project_type != "Research/POC" {
            needs
                .recommended_personas
                .push("User Experience".to_string());
        }

        if needs.recommended_personas.is_empty() {
            needs
                .recommended_personas
                .push("Project Manager".to_string());
        }

        // Deduplicate personas
        needs.recommended_personas.sort();
        needs.recommended_personas.dedup();

        // Recommend RAG domains based on tech stack
        for tech in &needs.tech_stack {
            if tech.contains("Rust") {
                needs.rag_domains.push("Rust".to_string());
            }
            if tech.contains("Vector") {
                needs.rag_domains.push("VectorDb".to_string());
            }
        }

        // Check for KV cache optimization opportunity
        if needs.pain_points.iter().any(|p| p.contains("Performance")) {
            needs.kv_cache_optimization = true;
        }

        Ok(needs)
    }

    /// Generate markdown drafts for proposed personas, skills, and scopes
    pub(crate) fn generate_proposals(&self, needs: &PersonaNeeds) -> Result<ProposedChanges> {
        let mut proposals = ProposedChanges {
            persona_drafts: Vec::new(),
            skill_drafts: Vec::new(),
            scope_drafts: Vec::new(),
            summary: format!(
                "I recommend {} personas based on your project type ({}) and team size ({}).",
                needs.recommended_personas.len(),
                needs.project_type,
                needs.team_size
            ),
        };

        // Generate persona markdown for each recommended persona
        // (In real implementation, these would be more detailed templates)
        for persona_name in &needs.recommended_personas {
            let markdown = format!(
                "# {}\n\n## Purpose\n{} for {} projects\n\n## Responsibilities\n- Primary: {}\n\n## Deliverables\n- Configuration and guidance\n\n## Operational Instructions\n1. Work collaboratively\n2. Document decisions\n\n## Interaction Matrix\n- Maestro: Handoff\n\n## Quality Criteria\n- All work must be validated\n- Errors must be logged\n",
                persona_name,
                persona_name,
                needs.project_type,
                persona_name
            );
            proposals.persona_drafts.push((
                format!("{}.md", slugify_persona_name(persona_name)),
                markdown,
            ));
        }

        // Generate scope template
        let scope_markdown = format!(
            "# 001-Project Setup\n\n## Objective\nInitialize project with {} configuration\n\n## Scope\n- Set up {} infrastructure\n- Document project structure\n- Prepare for development\n\n## Deliverables\n- Initial configuration\n- Team onboarding\n\n## Criteria\n- All components initialized\n- Team ready to start\n",
            needs.project_type, needs.tech_stack.join(", ")
        );
        proposals
            .scope_drafts
            .push(("001-project-setup.md".to_string(), scope_markdown));

        Ok(proposals)
    }

    /// Build a single-target directive draft from a directive-scoped session.
    ///
    /// This is the interview-driven authoring path that folds in the former
    /// form-based creation wizard for Create/Edit/Update operations. Delete is
    /// applied via the archive path, not by authoring, and is rejected here.
    pub fn build_directive_proposal(&self, session: &InterviewSession) -> Result<ProposedChanges> {
        let target = session
            .target
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("directive authoring requires a selected target"))?;

        validate_directive_operation(session.operation, target, session.target_file.as_deref())
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;

        if session.operation == DirectiveOperation::Delete {
            return Err(anyhow::anyhow!(
                "delete is applied via archive, not through interview authoring"
            ));
        }

        let answers: Vec<String> = session
            .exchange_history
            .iter()
            .map(|exchange| exchange.user_answer.trim().to_string())
            .filter(|answer| !answer.is_empty())
            .collect();

        let file_name = session
            .target_file
            .clone()
            .unwrap_or_else(|| target.default_file_name());
        let content = render_directive_markdown(
            target,
            session.operation,
            &answers,
            session.existing_content.as_deref(),
        );

        let mut proposal = ProposedChanges {
            persona_drafts: Vec::new(),
            skill_drafts: Vec::new(),
            scope_drafts: Vec::new(),
            summary: format!(
                "{} {} '{}'",
                session.operation.label(),
                target.kind_label(),
                target.display_name()
            ),
        };

        match target {
            DirectiveTarget::Persona { .. } => proposal.persona_drafts.push((file_name, content)),
            DirectiveTarget::Skill { .. } => proposal.skill_drafts.push((file_name, content)),
            DirectiveTarget::Scope { .. } => proposal.scope_drafts.push((file_name, content)),
        }

        Ok(proposal)
    }

    /// Run the Project-Manager-first scope authoring pipeline.
    ///
    /// Step 1 (Project Manager): write the scope file content from the captured
    /// answers. Step 2 (Maestro): read the written scope and derive the skill
    /// additions each non-Maestro persona needs. Maestro is never a derivation
    /// target. The pipeline also audits declared project dependencies and folds
    /// the required next actions into the Workspace hand-off.
    ///
    /// Derivation is heuristic and deterministic; no LLM synthesis is used.
    pub fn author_scope_with_additions(
        &self,
        session: &InterviewSession,
        project_deps: Option<&ProjectDepsConfig>,
    ) -> Result<ScopeAuthoringPlan> {
        let target = session
            .target
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("scope authoring requires a selected target"))?;

        if !matches!(target, DirectiveTarget::Scope { .. }) {
            return Err(anyhow::anyhow!(
                "scope authoring pipeline requires a scope target"
            ));
        }

        if session.operation == DirectiveOperation::Delete {
            return Err(anyhow::anyhow!(
                "delete is applied via archive, not through scope authoring"
            ));
        }

        // Step 1: the Project Manager writes the scope first.
        let answers: Vec<String> = session
            .exchange_history
            .iter()
            .map(|exchange| exchange.user_answer.trim().to_string())
            .filter(|answer| !answer.is_empty())
            .collect();

        let scope_file_name = session
            .target_file
            .clone()
            .unwrap_or_else(|| target.default_file_name());
        let scope_content = render_directive_markdown(
            target,
            session.operation,
            &answers,
            session.existing_content.as_deref(),
        );

        // Step 2: Maestro reads the written scope and derives persona additions.
        let objective = scope_objective(&scope_content);
        let content_lc = scope_content.to_lowercase();
        let additions: Vec<PersonaAddition> = ADDITION_TARGET_PERSONAS
            .iter()
            .filter(|persona| !is_maestro_identity(persona))
            .map(|persona| derive_scope_addition(persona, &objective, &content_lc))
            .collect();

        // Hand-off: Maestro audits dependencies and surfaces next actions.
        let mut next_actions = Vec::new();
        next_actions.push(format!(
            "Project Manager authored scope '{scope_file_name}'."
        ));
        for addition in &additions {
            next_actions.push(format!(
                "Add skill '{}' to {} ({}).",
                addition.skill_name, addition.persona, addition.rationale
            ));
        }
        next_actions.extend(dependency_audit_actions(project_deps));
        next_actions
            .push("Open Workspace monitor to run the sequential agent workflow.".to_string());

        Ok(ScopeAuthoringPlan {
            scope_file_name,
            scope_content,
            additions,
            next_actions,
        })
    }
}

/// Extract the scope objective (first non-empty line under `## Objective`).
fn scope_objective(scope_content: &str) -> String {
    let mut lines = scope_content.lines();
    while let Some(line) = lines.next() {
        if line.trim().eq_ignore_ascii_case("## Objective") {
            for next in lines.by_ref() {
                let trimmed = next.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }
    "the scope".to_string()
}

/// Deterministically derive a single skill addition for a non-Maestro persona by
/// reading the authored scope content.
fn derive_scope_addition(persona: &str, objective: &str, content_lc: &str) -> PersonaAddition {
    let (skill_name, capability, keywords): (&str, &str, &[&str]) = match persona {
        "Project Manager" => (
            "Scope Delivery Coordination",
            "Coordinate delivery of the scope across personas and milestones.",
            &[
                "milestone",
                "plan",
                "deliver",
                "coordinat",
                "timeline",
                "backlog",
            ],
        ),
        "Quality Assurance" => (
            "Acceptance Criteria Verification",
            "Verify the scope against its acceptance criteria.",
            &[
                "test",
                "quality",
                "accept",
                "verif",
                "validation",
                "regression",
            ],
        ),
        "User Experience" => (
            "Experience Validation",
            "Validate the user experience the scope delivers.",
            &[
                "ui",
                "ux",
                "interface",
                "experience",
                "usab",
                "frontend",
                "design",
            ],
        ),
        "Software Engineer" => (
            "Implementation Engineering",
            "Implement the technical work the scope requires.",
            &[
                "api",
                "code",
                "implement",
                "backend",
                "service",
                "build",
                "integration",
            ],
        ),
        _ => (
            "Scope Support",
            "Support delivery of the scope.",
            &["scope"],
        ),
    };

    let detected = keywords.iter().any(|keyword| content_lc.contains(keyword));
    let rationale = if detected {
        format!("scope '{objective}' shows {persona} signals")
    } else {
        format!("scope '{objective}' needs baseline {persona} coverage")
    };

    let target = DirectiveTarget::Skill {
        persona: persona.to_string(),
        name: skill_name.to_string(),
    };
    let answers = vec![
        capability.to_string(),
        format!("When delivering scope objective: {objective}."),
        "A verifiable contribution to the scope.".to_string(),
    ];
    let content = render_directive_markdown(&target, DirectiveOperation::Create, &answers, None);
    let file_name = format!("{}.md", slugify_persona_name(skill_name));

    PersonaAddition {
        persona: persona.to_string(),
        skill_name: skill_name.to_string(),
        rationale,
        file_name,
        content,
    }
}

/// Derive deterministic dependency-audit actions for the Workspace hand-off.
fn dependency_audit_actions(project_deps: Option<&ProjectDepsConfig>) -> Vec<String> {
    match project_deps {
        None => vec![
            "Maestro audit: declare project dependencies in maestro/project-deps.yaml.".to_string(),
        ],
        Some(config) => {
            let mut actions: Vec<String> = config
                .dependencies
                .iter()
                .filter(|dependency| dependency.required)
                .map(|dependency| {
                    let hint = dependency
                        .install_hint
                        .clone()
                        .unwrap_or_else(|| dependency.check_command.clone());
                    format!(
                        "Maestro audit: verify required dependency '{}' ({hint}).",
                        dependency.name
                    )
                })
                .collect();
            if actions.is_empty() {
                actions.push("Maestro audit: no required dependencies declared.".to_string());
            }
            actions
        }
    }
}

fn slugify_persona_name(name: &str) -> String {
    name.trim().to_lowercase().replace(' ', "-")
}

/// Render directive markdown for the interview-driven editor.
///
/// Edit and Create produce a full re-authored document; Update appends a
/// targeted section to the existing content (single-section change).
fn render_directive_markdown(
    target: &DirectiveTarget,
    operation: DirectiveOperation,
    answers: &[String],
    existing_content: Option<&str>,
) -> String {
    if operation == DirectiveOperation::Update {
        let base = existing_content.unwrap_or("").trim_end();
        let update_note = if answers.is_empty() {
            "No changes captured.".to_string()
        } else {
            answers
                .iter()
                .map(|answer| format!("- {answer}"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        if base.is_empty() {
            return format!("## Update\n{update_note}\n");
        }
        return format!("{base}\n\n## Update\n{update_note}\n");
    }

    let answer = |index: usize, fallback: &str| -> String {
        answers
            .get(index)
            .filter(|value| !value.is_empty())
            .cloned()
            .unwrap_or_else(|| fallback.to_string())
    };

    // Section headers are aligned with markdown governance required fields so the
    // authored documents pass validation when persisted.
    match target {
        DirectiveTarget::Persona { name } => format!(
            "# {name}\n\n## Purpose\n{}\n\n## Responsibilities\n{}\n\n## Deliverables\n{}\n\n## Operational Instructions\n{}\n\n## Interaction Matrix\n{}\n\n## Boundaries\n{}\n\n## Quality Criteria\n{}\n",
            answer(0, "Describe the persona's primary purpose."),
            answer(1, "List the persona's core responsibilities."),
            "Concrete artifacts and outputs this persona produces.",
            "1. Collaborate with peers.\n2. Document every decision.",
            answer(2, "Describe collaboration with other personas."),
            "Operate within Maestro governance and architecture boundaries.",
            answer(3, "Define the quality criteria for the persona's work."),
        ),
        DirectiveTarget::Skill { persona, name } => format!(
            "# {name}\n\n_Skill for the {persona} persona._\n\n## Objective\n{}\n\n## Triggers\n{}\n\n## Inputs\n{}\n\n## Outputs\n{}\n\n## Constraints\n{}\n",
            answer(0, "Describe the capability this skill adds."),
            answer(1, "Describe when the persona applies this skill."),
            "Relevant context, scope, and directives.",
            answer(2, "Describe a successful outcome."),
            "Respect Maestro governance and persona boundaries.",
        ),
        DirectiveTarget::Scope { name } => format!(
            "# {name}\n\n## Objective\n{}\n\n## Business Scope\n{}\n\n## Deliverables\n{}\n\n## Acceptance Criteria\n{}\n\n## Dependencies\n{}\n",
            answer(0, "State the objective of this scope."),
            answer(1, "List the work items in this scope."),
            "Artifacts produced by this scope.",
            answer(2, "List the acceptance criteria."),
            "External dependencies and prerequisites.",
        ),
    }
}

impl Default for InterviewBot {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interview_session_initializes_empty() {
        let session = InterviewSession::default();
        assert_eq!(session.turn_count, 0);
        assert!(session.exchange_history.is_empty());
        assert!(session.collected_needs.is_none());
        assert!(!session.approval_pending);
    }

    #[test]
    fn interview_bot_has_7_questions() {
        let bot = InterviewBot::new();
        assert_eq!(bot.question_templates.len(), 8);
        assert!(bot.get_question(1).is_some());
        assert!(bot.get_question(8).is_some());
        assert!(bot.get_question(9).is_none());
    }

    #[test]
    fn persona_needs_collects_pain_points() {
        let needs = PersonaNeeds {
            pain_points: vec!["Performance".to_string(), "Scalability".to_string()],
            ..Default::default()
        };
        assert_eq!(needs.pain_points.len(), 2);
    }

    fn exchange_with_answer(answer: &str) -> InterviewExchange {
        InterviewExchange {
            maestro_question: Uuid::new_v4(),
            maestro_text: "q".to_string(),
            user_answer: answer.to_string(),
            timestamp: SystemTime::now(),
        }
    }

    #[test]
    fn authoring_questions_are_kind_specific() {
        assert_eq!(
            DirectiveTarget::Persona {
                name: "Project Manager".to_string(),
            }
            .authoring_questions()
            .len(),
            4
        );
        assert_eq!(
            DirectiveTarget::Skill {
                persona: "Quality Assurance".to_string(),
                name: "regression-suite".to_string(),
            }
            .authoring_questions()
            .len(),
            3
        );
        assert_eq!(
            DirectiveTarget::Scope {
                name: "backend".to_string(),
            }
            .authoring_questions()
            .len(),
            3
        );
    }

    #[test]
    fn build_directive_proposal_creates_single_persona_draft() {
        let bot = InterviewBot::new();
        let mut session = InterviewSession::for_directive(
            DirectiveOperation::Create,
            DirectiveTarget::Persona {
                name: "Project Manager".to_string(),
            },
            None,
            None,
        )
        .expect("valid create session");
        session.exchange_history = vec![
            exchange_with_answer("Coordinate delivery"),
            exchange_with_answer("Plan and track milestones"),
        ];

        let proposal = bot
            .build_directive_proposal(&session)
            .expect("create proposal");
        assert_eq!(proposal.persona_drafts.len(), 1);
        assert!(proposal.skill_drafts.is_empty());
        assert!(proposal.scope_drafts.is_empty());
        let (file_name, content) = &proposal.persona_drafts[0];
        assert_eq!(file_name, "project-manager.md");
        assert!(content.contains("Coordinate delivery"));
        assert!(content.contains("## Responsibilities"));
    }

    #[test]
    fn build_directive_proposal_edit_scope_uses_target_file_and_full_rewrite() {
        let bot = InterviewBot::new();
        let mut session = InterviewSession::for_directive(
            DirectiveOperation::Edit,
            DirectiveTarget::Scope {
                name: "backend".to_string(),
            },
            Some("001-backend.md".to_string()),
            Some("# 001-backend\n\n## Objective\nold\n".to_string()),
        )
        .expect("valid edit session");
        session.exchange_history = vec![exchange_with_answer("Ship the API")];

        let proposal = bot
            .build_directive_proposal(&session)
            .expect("edit proposal");
        assert_eq!(proposal.scope_drafts.len(), 1);
        let (file_name, content) = &proposal.scope_drafts[0];
        assert_eq!(file_name, "001-backend.md");
        assert!(content.contains("Ship the API"));
        assert!(content.contains("## Acceptance Criteria"));
    }

    #[test]
    fn build_directive_proposal_update_appends_section_to_existing() {
        let bot = InterviewBot::new();
        let mut session = InterviewSession::for_directive(
            DirectiveOperation::Update,
            DirectiveTarget::Persona {
                name: "Quality Assurance".to_string(),
            },
            Some("quality-assurance.md".to_string()),
            Some("# Quality Assurance\n\n## Purpose\nkeep quality high\n".to_string()),
        )
        .expect("valid update session");
        session.exchange_history = vec![exchange_with_answer("Add exploratory testing")];

        let proposal = bot
            .build_directive_proposal(&session)
            .expect("update proposal");
        let (_, content) = &proposal.persona_drafts[0];
        assert!(content.contains("keep quality high"));
        assert!(content.contains("## Update"));
        assert!(content.contains("- Add exploratory testing"));
    }

    #[test]
    fn build_directive_proposal_rejects_delete() {
        let bot = InterviewBot::new();
        let session = InterviewSession::for_directive(
            DirectiveOperation::Delete,
            DirectiveTarget::Scope {
                name: "backend".to_string(),
            },
            Some("001-backend.md".to_string()),
            None,
        )
        .expect("valid delete session");
        assert!(bot.build_directive_proposal(&session).is_err());
    }

    #[test]
    fn build_directive_proposal_rejects_maestro_target() {
        let bot = InterviewBot::new();
        let session = InterviewSession {
            operation: DirectiveOperation::Edit,
            target: Some(DirectiveTarget::Persona {
                name: "Maestro".to_string(),
            }),
            target_file: Some("maestro.md".to_string()),
            ..InterviewSession::default()
        };
        assert!(bot.build_directive_proposal(&session).is_err());
    }
    #[test]
    fn directive_operation_exposes_labels_and_existing_target_rules() {
        assert_eq!(DirectiveOperation::Create.label(), "create");
        assert_eq!(DirectiveOperation::Edit.label(), "edit");
        assert_eq!(DirectiveOperation::Update.label(), "update");
        assert_eq!(DirectiveOperation::Delete.label(), "delete");
        assert!(!DirectiveOperation::Create.requires_existing_target());
        assert!(DirectiveOperation::Edit.requires_existing_target());
        assert!(DirectiveOperation::Update.requires_existing_target());
        assert!(DirectiveOperation::Delete.requires_existing_target());
    }

    #[test]
    fn directive_target_detects_maestro_identity_case_insensitively() {
        assert!(DirectiveTarget::Persona {
            name: "Maestro".to_string(),
        }
        .targets_maestro());
        assert!(DirectiveTarget::Skill {
            persona: "maestro".to_string(),
            name: "prompt-optimization".to_string(),
        }
        .targets_maestro());
        assert!(!DirectiveTarget::Persona {
            name: "Software Engineer".to_string(),
        }
        .targets_maestro());
        assert!(!DirectiveTarget::Scope {
            name: "backend".to_string(),
        }
        .targets_maestro());
    }

    #[test]
    fn validate_directive_operation_rejects_maestro_mutation() {
        for operation in [
            DirectiveOperation::Create,
            DirectiveOperation::Edit,
            DirectiveOperation::Update,
            DirectiveOperation::Delete,
        ] {
            let target = DirectiveTarget::Persona {
                name: "Maestro".to_string(),
            };
            let result = validate_directive_operation(operation, &target, Some("maestro.md"));
            assert_eq!(
                result,
                Err(DirectiveOperationError::ImmutableMaestro {
                    operation: operation.label(),
                })
            );
        }
    }

    #[test]
    fn validate_directive_operation_requires_existing_target_for_mutations() {
        let target = DirectiveTarget::Persona {
            name: "Project Manager".to_string(),
        };
        let result = validate_directive_operation(DirectiveOperation::Edit, &target, None);
        assert_eq!(
            result,
            Err(DirectiveOperationError::MissingExistingTarget { operation: "edit" })
        );
    }

    #[test]
    fn for_directive_builds_session_for_valid_non_maestro_operation() {
        let session = InterviewSession::for_directive(
            DirectiveOperation::Update,
            DirectiveTarget::Persona {
                name: "Quality Assurance".to_string(),
            },
            Some("quality-assurance.md".to_string()),
            Some("# Quality Assurance".to_string()),
        )
        .expect("valid non-maestro operation should build a session");

        assert_eq!(session.operation, DirectiveOperation::Update);
        assert_eq!(session.target_file.as_deref(), Some("quality-assurance.md"));
        assert!(session.target.is_some());
        assert_eq!(session.turn_count, 0);
    }

    #[test]
    fn for_directive_rejects_maestro_skill_target() {
        let result = InterviewSession::for_directive(
            DirectiveOperation::Delete,
            DirectiveTarget::Skill {
                persona: "Maestro".to_string(),
                name: "observability".to_string(),
            },
            Some("observability.md".to_string()),
            None,
        );

        assert_eq!(
            result.err(),
            Some(DirectiveOperationError::ImmutableMaestro {
                operation: "delete",
            })
        );
    }

    fn scope_session_with_answers() -> InterviewSession {
        let mut session = InterviewSession::for_directive(
            DirectiveOperation::Create,
            DirectiveTarget::Scope {
                name: "Checkout API".to_string(),
            },
            Some("001-checkout-api.md".to_string()),
            None,
        )
        .expect("valid scope session");
        session.exchange_history = vec![
            exchange_with_answer("Ship the checkout API and validate its acceptance tests"),
            exchange_with_answer("Implement backend service and a frontend interface"),
            exchange_with_answer("All acceptance criteria pass"),
        ];
        session
    }

    #[test]
    fn author_scope_writes_scope_first_and_derives_four_additions() {
        let bot = InterviewBot::new();
        let session = scope_session_with_answers();

        let plan = bot
            .author_scope_with_additions(&session, None)
            .expect("scope authoring plan");

        assert_eq!(plan.scope_file_name, "001-checkout-api.md");
        assert!(plan.scope_content.contains("Ship the checkout API"));
        assert!(plan.scope_content.contains("## Acceptance Criteria"));

        let personas: Vec<&str> = plan.additions.iter().map(|a| a.persona.as_str()).collect();
        assert_eq!(personas, ADDITION_TARGET_PERSONAS.to_vec());

        assert!(plan
            .next_actions
            .iter()
            .any(|action| action.contains("Project Manager authored scope")));
        assert!(plan
            .next_actions
            .iter()
            .any(|action| action.contains("Open Workspace monitor")));
    }

    #[test]
    fn author_scope_never_targets_maestro() {
        let bot = InterviewBot::new();
        let session = scope_session_with_answers();

        let plan = bot
            .author_scope_with_additions(&session, None)
            .expect("scope authoring plan");

        assert!(plan
            .additions
            .iter()
            .all(|addition| addition.persona != "Maestro"));
    }

    #[test]
    fn author_scope_additions_are_governance_valid_skills() {
        let bot = InterviewBot::new();
        let session = scope_session_with_answers();

        let plan = bot
            .author_scope_with_additions(&session, None)
            .expect("scope authoring plan");

        for addition in &plan.additions {
            for header in [
                "## Objective",
                "## Triggers",
                "## Inputs",
                "## Outputs",
                "## Constraints",
            ] {
                assert!(
                    addition.content.contains(header),
                    "addition for {} missing {header}",
                    addition.persona
                );
            }
        }
    }

    #[test]
    fn author_scope_audits_declared_required_dependencies() {
        use crate::application::project_deps::{ProjectDependency, ProjectDepsConfig};

        let bot = InterviewBot::new();
        let session = scope_session_with_answers();
        let deps = ProjectDepsConfig {
            dependencies: vec![
                ProjectDependency {
                    name: "cargo".to_string(),
                    check_command: "command -v cargo".to_string(),
                    required: true,
                    install_hint: Some("Install the Rust toolchain.".to_string()),
                },
                ProjectDependency {
                    name: "docker".to_string(),
                    check_command: "command -v docker".to_string(),
                    required: false,
                    install_hint: None,
                },
            ],
        };

        let plan = bot
            .author_scope_with_additions(&session, Some(&deps))
            .expect("scope authoring plan");

        assert!(plan
            .next_actions
            .iter()
            .any(|action| action.contains("required dependency 'cargo'")));
        assert!(plan
            .next_actions
            .iter()
            .all(|action| !action.contains("'docker'")));
    }

    #[test]
    fn author_scope_rejects_non_scope_target() {
        let bot = InterviewBot::new();
        let session = InterviewSession::for_directive(
            DirectiveOperation::Create,
            DirectiveTarget::Persona {
                name: "Project Manager".to_string(),
            },
            None,
            None,
        )
        .expect("valid persona session");

        assert!(bot.author_scope_with_additions(&session, None).is_err());
    }
}
