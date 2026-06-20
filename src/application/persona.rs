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
                product_persona(),
                engineering_persona(),
                ux_persona(),
                devops_persona(),
            ],
        }
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

fn product_persona() -> Persona {
    Persona {
        name: "Product".to_string(),
        purpose: "Turn business objectives into executable scope".to_string(),
        responsibilities: vec![
            "Define problem and priorities".to_string(),
            "Consolidate functional requirements".to_string(),
        ],
        deliverables: vec![
            "Prioritized delivery scope".to_string(),
            "Increment acceptance criteria".to_string(),
        ],
        operational_instructions: vec![
            "Validate business impact before approving scope".to_string(),
            "Synchronize handoff with Engineering and UX".to_string(),
        ],
        interaction_matrix: vec![
            PersonaInteraction {
                target_persona: "Engineering".to_string(),
                collaboration_contract: "Refine technical stories".to_string(),
                expected_handoff: "Prioritized backlog and acceptance criteria".to_string(),
            },
            PersonaInteraction {
                target_persona: "UX".to_string(),
                collaboration_contract: "Align target experience".to_string(),
                expected_handoff: "Journey objectives and user value".to_string(),
            },
            PersonaInteraction {
                target_persona: "DevOps".to_string(),
                collaboration_contract: "Plan release strategy".to_string(),
                expected_handoff: "Deployment window and risk".to_string(),
            },
        ],
        quality_criteria: vec![
            "Unambiguous scope".to_string(),
            "Measurable acceptance".to_string(),
        ],
    }
}

fn engineering_persona() -> Persona {
    Persona {
        name: "Engineering".to_string(),
        purpose: "Deliver a reliable technical solution".to_string(),
        responsibilities: vec![
            "Design delivery architecture".to_string(),
            "Implement and test with quality".to_string(),
        ],
        deliverables: vec![
            "Reviewed and tested code".to_string(),
            "Technical decision record".to_string(),
        ],
        operational_instructions: vec![
            "Report technical risks early".to_string(),
            "Maintain quality and observability standards".to_string(),
        ],
        interaction_matrix: vec![
            PersonaInteraction {
                target_persona: "Product".to_string(),
                collaboration_contract: "Clarify requirements".to_string(),
                expected_handoff: "Estimates and technical trade-offs".to_string(),
            },
            PersonaInteraction {
                target_persona: "UX".to_string(),
                collaboration_contract: "Enable experience goals".to_string(),
                expected_handoff: "Technical constraints and opportunities".to_string(),
            },
            PersonaInteraction {
                target_persona: "DevOps".to_string(),
                collaboration_contract: "Prepare pipeline and rollout".to_string(),
                expected_handoff: "Deployment and monitoring artifacts".to_string(),
            },
        ],
        quality_criteria: vec![
            "No functional regression".to_string(),
            "Adequate test coverage".to_string(),
        ],
    }
}

fn ux_persona() -> Persona {
    Persona {
        name: "UX".to_string(),
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
            "Align visual decisions with Product".to_string(),
        ],
        interaction_matrix: vec![
            PersonaInteraction {
                target_persona: "Product".to_string(),
                collaboration_contract: "Refine value proposition".to_string(),
                expected_handoff: "Prioritized usage scenarios".to_string(),
            },
            PersonaInteraction {
                target_persona: "Engineering".to_string(),
                collaboration_contract: "Detail interface behavior".to_string(),
                expected_handoff: "Implementable interaction specifications".to_string(),
            },
            PersonaInteraction {
                target_persona: "DevOps".to_string(),
                collaboration_contract: "Support progressive rollout".to_string(),
                expected_handoff: "Friction signals for monitoring".to_string(),
            },
        ],
        quality_criteria: vec![
            "Intuitive primary flow".to_string(),
            "Consistent experience".to_string(),
        ],
    }
}

fn devops_persona() -> Persona {
    Persona {
        name: "DevOps".to_string(),
        purpose: "Ensure reliable continuous delivery".to_string(),
        responsibilities: vec![
            "Automate build and release".to_string(),
            "Monitor operational health".to_string(),
        ],
        deliverables: vec![
            "Validated pipeline".to_string(),
            "Observability and rollback plan".to_string(),
        ],
        operational_instructions: vec![
            "Reduce deployment risk with gradual strategy".to_string(),
            "Ensure incident traceability".to_string(),
        ],
        interaction_matrix: vec![
            PersonaInteraction {
                target_persona: "Product".to_string(),
                collaboration_contract: "Plan release windows and impact".to_string(),
                expected_handoff: "Rollout status and risk".to_string(),
            },
            PersonaInteraction {
                target_persona: "Engineering".to_string(),
                collaboration_contract: "Standardize deployment operations".to_string(),
                expected_handoff: "Applied infrastructure requirements".to_string(),
            },
            PersonaInteraction {
                target_persona: "UX".to_string(),
                collaboration_contract: "Monitor experience friction".to_string(),
                expected_handoff: "Production usage metrics".to_string(),
            },
        ],
        quality_criteria: vec![
            "Reproducible deployment".to_string(),
            "Actionable observability".to_string(),
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
        assert_eq!(catalog.personas.len(), 4);
    }

    #[test]
    fn rejects_persona_without_interaction_matrix() {
        let invalid = Persona {
            name: "Product".to_string(),
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
                    name: "Engineering".to_string(),
                    purpose: "x".to_string(),
                    responsibilities: vec!["x".to_string()],
                    deliverables: vec!["x".to_string()],
                    operational_instructions: vec!["x".to_string()],
                    interaction_matrix: vec![PersonaInteraction {
                        target_persona: "Product".to_string(),
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
            }) if persona == "Product"
        ));
    }

    #[test]
    fn rejects_persona_with_invalid_interaction_target() {
        let invalid = Persona {
            name: "Product".to_string(),
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
                    name: "Engineering".to_string(),
                    purpose: "x".to_string(),
                    responsibilities: vec!["x".to_string()],
                    deliverables: vec!["x".to_string()],
                    operational_instructions: vec!["x".to_string()],
                    interaction_matrix: vec![PersonaInteraction {
                        target_persona: "Product".to_string(),
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
            if persona == "Product" && target == "MissingPersona"
        ));
    }
}
