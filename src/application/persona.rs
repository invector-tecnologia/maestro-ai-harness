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

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PersonaParseError {
    #[error("Persona markdown is missing a '# <Name>' title")]
    MissingTitle,
    #[error("Persona markdown is missing required section: {section}")]
    MissingSection { section: String },
    #[error("Persona markdown section is empty: {section}")]
    EmptySection { section: String },
    #[error("Malformed interaction entry (expected 'Target | Contract | Handoff'): {entry}")]
    MalformedInteraction { entry: String },
}

#[derive(Debug, Error)]
pub enum PersonaCatalogLoadError {
    #[error("No persona markdown files found in governance directory")]
    Empty,
    #[error("Failed to read persona governance: {0}")]
    Governance(#[from] crate::application::markdown_governance::MarkdownGovernanceError),
    #[error("Failed to parse persona markdown ({file}): {source}")]
    Parse {
        file: String,
        #[source]
        source: PersonaParseError,
    },
    #[error("Persona catalog validation failed: {0}")]
    Validation(#[from] PersonaError),
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

    /// Render this persona as canonical governance markdown.
    ///
    /// The schema is the single source of truth shared by the runtime catalog,
    /// the Core Mode editor, and `maestro scaffold-markdown`.
    pub fn to_markdown(&self) -> String {
        let bullet_block = |items: &[String]| -> String {
            items
                .iter()
                .map(|item| format!("- {item}"))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let interactions = self
            .interaction_matrix
            .iter()
            .map(|interaction| {
                format!(
                    "- {} | {} | {}",
                    interaction.target_persona,
                    interaction.collaboration_contract,
                    interaction.expected_handoff
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "# {name}\n\n## Purpose\n{purpose}\n\n## Responsibilities\n{responsibilities}\n\n## Deliverables\n{deliverables}\n\n## Operational Instructions\n{instructions}\n\n## Interaction Matrix\n{interactions}\n\n## Quality Criteria\n{quality}\n",
            name = self.name,
            purpose = self.purpose,
            responsibilities = bullet_block(&self.responsibilities),
            deliverables = bullet_block(&self.deliverables),
            instructions = bullet_block(&self.operational_instructions),
            interactions = interactions,
            quality = bullet_block(&self.quality_criteria),
        )
    }

    /// Parse a persona from canonical governance markdown.
    ///
    /// Performs structural parsing only; cross-persona interaction targets are
    /// validated by [`PersonaCatalog::validate`]. Returns a typed error on
    /// malformed input so callers never panic on hand-edited files.
    pub fn from_markdown(content: &str) -> Result<Self, PersonaParseError> {
        let mut name: Option<String> = None;
        let mut current_section: Option<String> = None;
        let mut sections: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        for raw_line in content.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some(heading) = line.strip_prefix("## ") {
                let key = heading.trim().to_lowercase();
                current_section = Some(key.clone());
                sections.entry(key).or_default();
                continue;
            }

            if let Some(title) = line.strip_prefix("# ") {
                if name.is_none() {
                    name = Some(title.trim().to_string());
                }
                continue;
            }

            if let Some(section) = &current_section {
                let value = line.strip_prefix("- ").unwrap_or(line).trim().to_string();
                if !value.is_empty() {
                    sections.entry(section.clone()).or_default().push(value);
                }
            }
        }

        let name = name.ok_or(PersonaParseError::MissingTitle)?;

        let take = |section: &str| -> Result<Vec<String>, PersonaParseError> {
            match sections.get(section) {
                Some(values) if !values.is_empty() => Ok(values.clone()),
                Some(_) => Err(PersonaParseError::EmptySection {
                    section: section.to_string(),
                }),
                None => Err(PersonaParseError::MissingSection {
                    section: section.to_string(),
                }),
            }
        };

        let purpose = take("purpose")?
            .first()
            .cloned()
            .ok_or(PersonaParseError::EmptySection {
                section: "purpose".to_string(),
            })?;
        let responsibilities = take("responsibilities")?;
        let deliverables = take("deliverables")?;
        let operational_instructions = take("operational instructions")?;
        let quality_criteria = take("quality criteria")?;

        let mut interaction_matrix = Vec::new();
        for entry in take("interaction matrix")? {
            let parts: Vec<&str> = entry.split('|').map(|part| part.trim()).collect();
            if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
                return Err(PersonaParseError::MalformedInteraction { entry });
            }
            interaction_matrix.push(PersonaInteraction {
                target_persona: parts[0].to_string(),
                collaboration_contract: parts[1].to_string(),
                expected_handoff: parts[2].to_string(),
            });
        }

        Ok(Self {
            name,
            purpose,
            responsibilities,
            deliverables,
            operational_instructions,
            interaction_matrix,
            quality_criteria,
        })
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

    /// Resolve the runtime persona catalog from governed markdown, falling back
    /// to the in-code defaults when governance is empty, missing, or invalid.
    ///
    /// This is the single resolution point that makes Core Mode persona edits
    /// drive the runtime agent set while never panicking on a malformed file.
    pub fn from_governance(
        governance: &crate::application::markdown_governance::MarkdownGovernance,
    ) -> Self {
        match Self::try_from_governance(governance) {
            Ok(catalog) => catalog,
            Err(error) => {
                tracing::warn!(
                    %error,
                    "falling back to in-code default personas; governed catalog unavailable"
                );
                Self::default_personas()
            }
        }
    }

    /// Build and validate the catalog strictly from governed markdown.
    ///
    /// The immutable Maestro orchestrator is always overridden with its trusted
    /// in-code definition so on-disk edits cannot weaken its governance role.
    pub fn try_from_governance(
        governance: &crate::application::markdown_governance::MarkdownGovernance,
    ) -> Result<Self, PersonaCatalogLoadError> {
        let file_names = governance.list_personas()?;
        if file_names.is_empty() {
            return Err(PersonaCatalogLoadError::Empty);
        }

        let mut personas = Vec::with_capacity(file_names.len());
        for file_name in file_names {
            let path = governance.personas_dir().join(&file_name);
            let content = governance.read_document(&path)?;
            let persona = Persona::from_markdown(&content).map_err(|source| {
                PersonaCatalogLoadError::Parse {
                    file: file_name,
                    source,
                }
            })?;
            personas.push(persona);
        }

        // Guarantee the immutable Maestro orchestrator is present and trusted.
        let maestro = maestro_persona();
        personas.retain(|persona| persona.name != maestro.name);
        personas.insert(0, maestro);

        let catalog = Self { personas };
        catalog.validate()?;
        Ok(catalog)
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
    fn persona_markdown_round_trips_for_every_default_persona() {
        for persona in PersonaCatalog::default_personas().personas {
            let markdown = persona.to_markdown();
            let parsed = Persona::from_markdown(&markdown)
                .expect("canonical persona markdown must parse back into a persona");
            assert_eq!(parsed, persona);
        }
    }

    #[test]
    fn from_markdown_parses_structured_interaction_matrix() {
        let markdown = "# Project Manager\n\n## Purpose\nCoordinate delivery\n\n## Responsibilities\n- Sequence milestones\n\n## Deliverables\n- Backlog\n\n## Operational Instructions\n- Keep scope small\n\n## Interaction Matrix\n- Maestro | Report status | Risk notes\n\n## Quality Criteria\n- Measurable acceptance\n";

        let persona = Persona::from_markdown(markdown).expect("valid markdown parses");

        assert_eq!(persona.name, "Project Manager");
        assert_eq!(persona.purpose, "Coordinate delivery");
        assert_eq!(persona.interaction_matrix.len(), 1);
        assert_eq!(persona.interaction_matrix[0].target_persona, "Maestro");
        assert_eq!(
            persona.interaction_matrix[0].collaboration_contract,
            "Report status"
        );
        assert_eq!(persona.interaction_matrix[0].expected_handoff, "Risk notes");
    }

    #[test]
    fn from_markdown_rejects_missing_title() {
        let markdown = "## Purpose\nNo title here\n";

        let result = Persona::from_markdown(markdown);

        assert_eq!(result, Err(PersonaParseError::MissingTitle));
    }

    #[test]
    fn from_markdown_rejects_missing_section() {
        let markdown = "# Solo\n\n## Purpose\nExists\n";

        let result = Persona::from_markdown(markdown);

        assert_eq!(
            result,
            Err(PersonaParseError::MissingSection {
                section: "responsibilities".to_string(),
            })
        );
    }

    #[test]
    fn from_markdown_rejects_malformed_interaction() {
        let markdown = "# Solo\n\n## Purpose\nExists\n\n## Responsibilities\n- Do work\n\n## Deliverables\n- Output\n\n## Operational Instructions\n- Be precise\n\n## Interaction Matrix\n- Maestro only two parts\n\n## Quality Criteria\n- Traceable\n";

        let result = Persona::from_markdown(markdown);

        assert_eq!(
            result,
            Err(PersonaParseError::MalformedInteraction {
                entry: "Maestro only two parts".to_string(),
            })
        );
    }

    fn temp_governance_with_workers() -> (
        std::path::PathBuf,
        crate::application::markdown_governance::MarkdownGovernance,
    ) {
        use crate::application::markdown_governance::MarkdownGovernance;
        let root = std::env::temp_dir().join(format!("maestro-persona-{}", uuid::Uuid::new_v4()));
        let governance = MarkdownGovernance::new(&root);
        governance
            .ensure_directories()
            .expect("ensure governance directories");
        for persona in [
            project_manager_persona(),
            quality_assurance_persona(),
            user_experience_persona(),
            software_engineer_persona(),
        ] {
            let file = governance.personas_dir().join(format!(
                "{}.md",
                persona.name.to_lowercase().replace(' ', "-")
            ));
            std::fs::write(file, persona.to_markdown()).expect("write worker persona");
        }
        (root, governance)
    }

    #[test]
    fn from_governance_loads_workers_and_injects_immutable_maestro() {
        let (root, governance) = temp_governance_with_workers();

        let catalog = PersonaCatalog::from_governance(&governance);

        assert!(catalog.validate().is_ok());
        assert_eq!(catalog.personas.len(), 5);
        assert_eq!(catalog.personas[0].name, "Maestro");
        assert_eq!(catalog.personas[0], maestro_persona());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn from_governance_falls_back_to_defaults_when_empty() {
        use crate::application::markdown_governance::MarkdownGovernance;
        let root = std::env::temp_dir().join(format!("maestro-persona-{}", uuid::Uuid::new_v4()));
        let governance = MarkdownGovernance::new(&root);
        governance
            .ensure_directories()
            .expect("ensure governance directories");

        let catalog = PersonaCatalog::from_governance(&governance);

        assert_eq!(catalog, PersonaCatalog::default_personas());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn from_governance_overrides_on_disk_maestro_tampering() {
        let (root, governance) = temp_governance_with_workers();
        let mut tampered = maestro_persona();
        tampered.purpose = "Tampered weakened purpose".to_string();
        std::fs::write(
            governance.personas_dir().join("maestro.md"),
            tampered.to_markdown(),
        )
        .expect("write tampered maestro");

        let catalog = PersonaCatalog::from_governance(&governance);

        let maestro = catalog
            .personas
            .iter()
            .find(|persona| persona.name == "Maestro")
            .expect("Maestro present");
        assert_eq!(*maestro, maestro_persona());
        assert_ne!(maestro.purpose, "Tampered weakened purpose");

        let _ = std::fs::remove_dir_all(root);
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
