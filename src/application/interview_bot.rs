use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::SystemTime;
use thiserror::Error;
use uuid::Uuid;

use crate::application::persona_operations::PersonaRuntimeRole;

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
}

fn slugify_persona_name(name: &str) -> String {
    name.trim().to_lowercase().replace(' ', "-")
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
}
