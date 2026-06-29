use std::collections::HashSet;

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Persona {
    pub name: String,
    pub purpose: String,
    pub responsibilities: Vec<String>,
    pub deliverables: Vec<String>,
    pub operational_instructions: Vec<String>,
    pub interaction_matrix: Vec<PersonaInteraction>,
    pub quality_criteria: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonaInteraction {
    pub target_persona: String,
    pub collaboration_contract: String,
    pub expected_handoff: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonaCatalog {
    pub personas: Vec<Persona>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PersonaError {
    #[error("Missing required field in persona {persona}: {field}")]
    MissingRequiredField {
        persona: String,
        field: &'static str,
    },
    #[error("Duplicate persona in catalog: {0}")]
    DuplicatePersona(String),
    #[error("Invalid interaction in {persona}: unknown target {target}")]
    UnknownInteractionTarget { persona: String, target: String },
    #[error("Invalid interaction in {persona}: self-loop is not allowed")]
    SelfInteraction { persona: String },
    #[error("Duplicate interaction in {persona} for {target}")]
    DuplicateInteraction { persona: String, target: String },
}

impl Persona {
    pub fn validate(&self, known_personas: &HashSet<String>) -> Result<(), PersonaError> {
        validate_non_empty(&self.name, &self.name, "identity")?;
        validate_non_empty(&self.name, &self.purpose, "purpose")?;
        validate_non_empty_vec(&self.name, &self.responsibilities, "responsibilities")?;
        validate_non_empty_vec(&self.name, &self.deliverables, "deliverables")?;
        validate_non_empty_vec(
            &self.name,
            &self.operational_instructions,
            "operational_instructions",
        )?;
        validate_non_empty_vec(&self.name, &self.quality_criteria, "quality_criteria")?;

        if self.interaction_matrix.is_empty() {
            return Err(PersonaError::MissingRequiredField {
                persona: self.name.clone(),
                field: "interaction_matrix",
            });
        }

        let mut targets = HashSet::new();
        for interaction in &self.interaction_matrix {
            validate_non_empty(
                &self.name,
                &interaction.target_persona,
                "interaction.target",
            )?;
            validate_non_empty(
                &self.name,
                &interaction.collaboration_contract,
                "interaction.collaboration_contract",
            )?;
            validate_non_empty(
                &self.name,
                &interaction.expected_handoff,
                "interaction.expected_handoff",
            )?;

            if interaction.target_persona == self.name {
                return Err(PersonaError::SelfInteraction {
                    persona: self.name.clone(),
                });
            }

            if !known_personas.contains(&interaction.target_persona) {
                return Err(PersonaError::UnknownInteractionTarget {
                    persona: self.name.clone(),
                    target: interaction.target_persona.clone(),
                });
            }

            if !targets.insert(interaction.target_persona.clone()) {
                return Err(PersonaError::DuplicateInteraction {
                    persona: self.name.clone(),
                    target: interaction.target_persona.clone(),
                });
            }
        }

        Ok(())
    }
}

impl PersonaCatalog {
    pub fn validate(&self) -> Result<(), PersonaError> {
        let mut unique_names = HashSet::new();
        for persona in &self.personas {
            if persona.name.trim().is_empty() {
                return Err(PersonaError::MissingRequiredField {
                    persona: "<missing_name>".to_string(),
                    field: "identity",
                });
            }

            if !unique_names.insert(persona.name.clone()) {
                return Err(PersonaError::DuplicatePersona(persona.name.clone()));
            }
        }

        for persona in &self.personas {
            persona.validate(&unique_names)?;
        }

        Ok(())
    }

    pub fn default_personas() -> Self {
        Self {
            personas: vec![
                maestro_persona(),
                project_manager_persona(),
                quality_assurance_persona(),
                user_experience_persona(),
                software_engineer_persona(),
            ],
        }
    }
}

fn maestro_persona() -> Persona {
    Persona {
        name: "Maestro".to_string(),
        purpose: "Rule software-house directives and orchestrate persona operations".to_string(),
        responsibilities: vec![
            "Call persona creation and persona updates".to_string(),
            "Call persona skill creation and skill updates".to_string(),
            "Call project scope creation and scope updates".to_string(),
        ],
        deliverables: vec![
            "Governed directive plans for personas, skills, and scopes".to_string(),
            "Interview synthesis with actionable operation handoffs".to_string(),
        ],
        operational_instructions: vec![
            "Optimize prompts for precision, context efficiency, and deterministic outcomes"
                .to_string(),
            "Apply product creation and launching strategy thinking to every directive decision"
                .to_string(),
            "Operate as software-house manager while preserving architecture boundaries"
                .to_string(),
            "Instrument telemetry and observability strategy for operational transparency"
                .to_string(),
            "Apply SERP optimization strategy for discoverability-oriented artifacts".to_string(),
            "Enforce Clean Architecture and Extreme Programming execution discipline".to_string(),
            "Run planning and delivery using Agile and Lean strategy heuristics".to_string(),
        ],
        interaction_matrix: vec![
            PersonaInteraction {
                target_persona: "Project Manager".to_string(),
                collaboration_contract: "Pass approved intent and directive priorities".to_string(),
                expected_handoff: "Prioritized scope and acceptance strategy".to_string(),
            },
            PersonaInteraction {
                target_persona: "Quality Assurance".to_string(),
                collaboration_contract: "Inject quality gates and verification depth".to_string(),
                expected_handoff: "Risk matrix and coverage criteria".to_string(),
            },
            PersonaInteraction {
                target_persona: "User Experience".to_string(),
                collaboration_contract: "Align operating directives to user-centered outcomes"
                    .to_string(),
                expected_handoff: "Experience strategy and interaction guardrails".to_string(),
            },
            PersonaInteraction {
                target_persona: "Software Engineer".to_string(),
                collaboration_contract: "Translate strategy into implementation constraints"
                    .to_string(),
                expected_handoff: "Architecture decisions and delivery increments".to_string(),
            },
        ],
        quality_criteria: vec![
            "Directive governance decisions are traceable and auditable".to_string(),
            "Prompt outcomes remain precise, reusable, and context-efficient".to_string(),
        ],
    }
}

fn validate_non_empty(persona: &str, value: &str, field: &'static str) -> Result<(), PersonaError> {
    if value.trim().is_empty() {
        return Err(PersonaError::MissingRequiredField {
            persona: persona.to_string(),
            field,
        });
    }
    Ok(())
}

fn validate_non_empty_vec(
    persona: &str,
    values: &[String],
    field: &'static str,
) -> Result<(), PersonaError> {
    if values.is_empty() || values.iter().any(|v| v.trim().is_empty()) {
        return Err(PersonaError::MissingRequiredField {
            persona: persona.to_string(),
            field,
        });
    }
    Ok(())
}

fn project_manager_persona() -> Persona {
    Persona {
        name: "Project Manager".to_string(),
        purpose: "Convert product goals into coordinated delivery directives".to_string(),
        responsibilities: vec![
            "Define roadmap priorities and milestone sequencing".to_string(),
            "Coordinate scope boundaries with quality and engineering".to_string(),
        ],
        deliverables: vec![
            "Prioritized milestone backlog".to_string(),
            "Acceptance readiness checklist".to_string(),
        ],
        operational_instructions: vec![
            "Validate launch value and execution feasibility before approving changes".to_string(),
            "Keep scope increments small, testable, and reversible".to_string(),
        ],
        interaction_matrix: vec![
            PersonaInteraction {
                target_persona: "Maestro".to_string(),
                collaboration_contract: "Report delivery status and ask for directive arbitration"
                    .to_string(),
                expected_handoff: "Scope deltas and delivery risk notes".to_string(),
            },
            PersonaInteraction {
                target_persona: "User Experience".to_string(),
                collaboration_contract: "Align user value objectives per increment".to_string(),
                expected_handoff: "Journey goals and usability constraints".to_string(),
            },
            PersonaInteraction {
                target_persona: "Software Engineer".to_string(),
                collaboration_contract: "Negotiate implementation sequencing and trade-offs"
                    .to_string(),
                expected_handoff: "Delivery plan and technical constraints".to_string(),
            },
        ],
        quality_criteria: vec![
            "Scope is unambiguous and launch-oriented".to_string(),
            "Acceptance criteria are measurable".to_string(),
        ],
    }
}

fn quality_assurance_persona() -> Persona {
    Persona {
        name: "Quality Assurance".to_string(),
        purpose: "Protect delivery quality through verification strategy".to_string(),
        responsibilities: vec![
            "Design test strategy and coverage depth".to_string(),
            "Validate release readiness with objective evidence".to_string(),
        ],
        deliverables: vec![
            "Risk-based test plan".to_string(),
            "Quality gate report".to_string(),
        ],
        operational_instructions: vec![
            "Automate regressions where risk justifies investment".to_string(),
            "Block releases lacking acceptance evidence".to_string(),
        ],
        interaction_matrix: vec![
            PersonaInteraction {
                target_persona: "Maestro".to_string(),
                collaboration_contract: "Escalate unresolved quality risks".to_string(),
                expected_handoff: "Go/No-Go recommendation with evidence".to_string(),
            },
            PersonaInteraction {
                target_persona: "Project Manager".to_string(),
                collaboration_contract: "Align acceptance criteria to milestone outcomes"
                    .to_string(),
                expected_handoff: "Coverage gaps and mitigation priorities".to_string(),
            },
            PersonaInteraction {
                target_persona: "Software Engineer".to_string(),
                collaboration_contract: "Drive defect prevention and testability improvements"
                    .to_string(),
                expected_handoff: "Defect findings and hardening recommendations".to_string(),
            },
        ],
        quality_criteria: vec![
            "Critical paths are covered by repeatable checks".to_string(),
            "Quality decisions are evidence-backed".to_string(),
        ],
    }
}

fn user_experience_persona() -> Persona {
    Persona {
        name: "User Experience".to_string(),
        purpose: "Ensure experience clarity and usability".to_string(),
        responsibilities: vec![
            "Design flows and interfaces".to_string(),
            "Validate interaction consistency".to_string(),
        ],
        deliverables: vec![
            "Interaction specification".to_string(),
            "Usability checklist".to_string(),
        ],
        operational_instructions: vec![
            "Anticipate friction in user flows".to_string(),
            "Align visual decisions with Project Manager priorities".to_string(),
        ],
        interaction_matrix: vec![
            PersonaInteraction {
                target_persona: "Maestro".to_string(),
                collaboration_contract: "Report friction patterns requiring directive changes"
                    .to_string(),
                expected_handoff: "Experience findings and redesign proposals".to_string(),
            },
            PersonaInteraction {
                target_persona: "Project Manager".to_string(),
                collaboration_contract: "Refine value proposition".to_string(),
                expected_handoff: "Prioritized usage scenarios".to_string(),
            },
            PersonaInteraction {
                target_persona: "Software Engineer".to_string(),
                collaboration_contract: "Detail interface behavior".to_string(),
                expected_handoff: "Implementable interaction specifications".to_string(),
            },
        ],
        quality_criteria: vec![
            "Intuitive primary flow".to_string(),
            "Consistent experience".to_string(),
        ],
    }
}

fn software_engineer_persona() -> Persona {
    Persona {
        name: "Software Engineer".to_string(),
        purpose: "Implement architecture safely with language-agnostic engineering practices"
            .to_string(),
        responsibilities: vec![
            "Design and implement modular solutions".to_string(),
            "Maintain testing, observability, and maintainability baselines".to_string(),
        ],
        deliverables: vec![
            "Working software increments".to_string(),
            "Technical documentation and operational runbooks".to_string(),
        ],
        operational_instructions: vec![
            "Apply language-agnostic clean code and test-first discipline".to_string(),
            "Surface trade-offs early and preserve architecture boundaries".to_string(),
        ],
        interaction_matrix: vec![
            PersonaInteraction {
                target_persona: "Maestro".to_string(),
                collaboration_contract: "Request directive arbitration for architecture conflicts"
                    .to_string(),
                expected_handoff: "Implementation status and decision proposals".to_string(),
            },
            PersonaInteraction {
                target_persona: "Project Manager".to_string(),
                collaboration_contract: "Align implementation sequence with milestone scope"
                    .to_string(),
                expected_handoff: "Effort estimates and dependency risks".to_string(),
            },
            PersonaInteraction {
                target_persona: "Quality Assurance".to_string(),
                collaboration_contract: "Increase testability and defect prevention".to_string(),
                expected_handoff: "Build artifacts and verification hooks".to_string(),
            },
        ],
        quality_criteria: vec![
            "Implementation remains language-agnostic and maintainable".to_string(),
            "Regression risk is controlled by automated verification".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_default_persona_catalog() {
        let catalog = PersonaCatalog::default_personas();

        let result = catalog.validate();

        assert!(result.is_ok());
        assert_eq!(catalog.personas.len(), 5);
    }

    #[test]
    fn rejects_persona_without_interaction_matrix() {
        let invalid = Persona {
            name: "Project Manager".to_string(),
            purpose: "x".to_string(),
            responsibilities: vec!["x".to_string()],
            deliverables: vec!["x".to_string()],
            operational_instructions: vec!["x".to_string()],
            interaction_matrix: vec![],
            quality_criteria: vec!["x".to_string()],
        };

        let catalog = PersonaCatalog {
            personas: vec![
                invalid,
                Persona {
                    name: "Software Engineer".to_string(),
                    purpose: "x".to_string(),
                    responsibilities: vec!["x".to_string()],
                    deliverables: vec!["x".to_string()],
                    operational_instructions: vec!["x".to_string()],
                    interaction_matrix: vec![PersonaInteraction {
                        target_persona: "Project Manager".to_string(),
                        collaboration_contract: "x".to_string(),
                        expected_handoff: "x".to_string(),
                    }],
                    quality_criteria: vec!["x".to_string()],
                },
            ],
        };

        let result = catalog.validate();

        assert!(matches!(
            result,
            Err(PersonaError::MissingRequiredField {
                persona,
                field: "interaction_matrix"
            }) if persona == "Project Manager"
        ));
    }

    #[test]
    fn rejects_persona_with_invalid_interaction_target() {
        let invalid = Persona {
            name: "Project Manager".to_string(),
            purpose: "x".to_string(),
            responsibilities: vec!["x".to_string()],
            deliverables: vec!["x".to_string()],
            operational_instructions: vec!["x".to_string()],
            interaction_matrix: vec![PersonaInteraction {
                target_persona: "MissingPersona".to_string(),
                collaboration_contract: "x".to_string(),
                expected_handoff: "x".to_string(),
            }],
            quality_criteria: vec!["x".to_string()],
        };

        let catalog = PersonaCatalog {
            personas: vec![
                invalid,
                Persona {
                    name: "Software Engineer".to_string(),
                    purpose: "x".to_string(),
                    responsibilities: vec!["x".to_string()],
                    deliverables: vec!["x".to_string()],
                    operational_instructions: vec!["x".to_string()],
                    interaction_matrix: vec![PersonaInteraction {
                        target_persona: "Project Manager".to_string(),
                        collaboration_contract: "x".to_string(),
                        expected_handoff: "x".to_string(),
                    }],
                    quality_criteria: vec!["x".to_string()],
                },
            ],
        };

        let result = catalog.validate();

        assert!(matches!(
            result,
            Err(PersonaError::UnknownInteractionTarget { persona, target })
            if persona == "Project Manager" && target == "MissingPersona"
        ));
    }
}
