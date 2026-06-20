use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::SystemTime;
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

/// Collected project needs extracted from interview responses
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PersonaNeeds {
    pub project_name: String,
    pub project_type: String, // e.g., "SaaS", "CLI Tool", "Research Project"
    pub team_size: String,    // e.g., "Solo", "Small (3-5)", "Large (10+)"
    pub pain_points: Vec<String>,
    pub tech_stack: Vec<String>,
    pub recommended_personas: Vec<String>, // e.g., ["Product", "Engineering", "DevOps"]
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
        }
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
            needs.recommended_personas.push("Engineering".to_string());
        }
        if needs.project_type.contains("SaaS") {
            needs.recommended_personas.push("Product".to_string());
        }
        if needs.pain_points.iter().any(|p| p.contains("Scalability")) {
            needs.recommended_personas.push("DevOps".to_string());
        }
        if needs.project_type != "Research/POC" {
            needs.recommended_personas.push("UX".to_string());
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
            proposals
                .persona_drafts
                .push((format!("{}.md", persona_name.to_lowercase()), markdown));
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
}
