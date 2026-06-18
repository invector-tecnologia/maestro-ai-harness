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
    #[error("Campo obrigatorio ausente na persona {persona}: {field}")]
    MissingRequiredField {
        persona: String,
        field: &'static str,
    },
    #[error("Persona duplicada no catalogo: {0}")]
    DuplicatePersona(String),
    #[error("Interacao invalida em {persona}: destino inexistente {target}")]
    UnknownInteractionTarget { persona: String, target: String },
    #[error("Interacao invalida em {persona}: self-loop nao permitido")]
    SelfInteraction { persona: String },
    #[error("Interacao duplicada em {persona} para {target}")]
    DuplicateInteraction { persona: String, target: String },
}

impl Persona {
    pub fn validate(&self, known_personas: &HashSet<String>) -> Result<(), PersonaError> {
        validate_non_empty(&self.name, &self.name, "identidade")?;
        validate_non_empty(&self.name, &self.purpose, "proposito")?;
        validate_non_empty_vec(&self.name, &self.responsibilities, "responsabilidades")?;
        validate_non_empty_vec(&self.name, &self.deliverables, "entregaveis")?;
        validate_non_empty_vec(
            &self.name,
            &self.operational_instructions,
            "instrucoes_operacionais",
        )?;
        validate_non_empty_vec(&self.name, &self.quality_criteria, "criterios_qualidade")?;

        if self.interaction_matrix.is_empty() {
            return Err(PersonaError::MissingRequiredField {
                persona: self.name.clone(),
                field: "matriz_de_interacao",
            });
        }

        let mut targets = HashSet::new();
        for interaction in &self.interaction_matrix {
            validate_non_empty(&self.name, &interaction.target_persona, "interacao.destino")?;
            validate_non_empty(
                &self.name,
                &interaction.collaboration_contract,
                "interacao.contrato",
            )?;
            validate_non_empty(
                &self.name,
                &interaction.expected_handoff,
                "interacao.handoff",
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
                    persona: "<sem_nome>".to_string(),
                    field: "identidade",
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
        purpose: "Transformar objetivos de negocio em escopo executavel".to_string(),
        responsibilities: vec![
            "Definir problema e prioridades".to_string(),
            "Consolidar requisitos funcionais".to_string(),
        ],
        deliverables: vec![
            "Escopo de entrega priorizado".to_string(),
            "Criterios de aceite do incremento".to_string(),
        ],
        operational_instructions: vec![
            "Validar impacto de negocio antes de aprovar escopo".to_string(),
            "Sincronizar handoff com Engineering e UX".to_string(),
        ],
        interaction_matrix: vec![
            PersonaInteraction {
                target_persona: "Engineering".to_string(),
                collaboration_contract: "Refinar historias tecnicas".to_string(),
                expected_handoff: "Backlog priorizado e criterios de aceite".to_string(),
            },
            PersonaInteraction {
                target_persona: "UX".to_string(),
                collaboration_contract: "Alinhar experiencia alvo".to_string(),
                expected_handoff: "Objetivos de jornada e valor ao usuario".to_string(),
            },
            PersonaInteraction {
                target_persona: "DevOps".to_string(),
                collaboration_contract: "Planejar estrategia de release".to_string(),
                expected_handoff: "Janela e risco de deploy".to_string(),
            },
        ],
        quality_criteria: vec![
            "Escopo sem ambiguidades".to_string(),
            "Aceite mensuravel".to_string(),
        ],
    }
}

fn engineering_persona() -> Persona {
    Persona {
        name: "Engineering".to_string(),
        purpose: "Entregar solucao tecnica confiavel".to_string(),
        responsibilities: vec![
            "Projetar arquitetura da entrega".to_string(),
            "Implementar e testar com qualidade".to_string(),
        ],
        deliverables: vec![
            "Codigo revisado e testado".to_string(),
            "Documento tecnico de decisao".to_string(),
        ],
        operational_instructions: vec![
            "Reportar riscos tecnicos cedo".to_string(),
            "Manter padroes de qualidade e observabilidade".to_string(),
        ],
        interaction_matrix: vec![
            PersonaInteraction {
                target_persona: "Product".to_string(),
                collaboration_contract: "Clarificar requisitos".to_string(),
                expected_handoff: "Estimativas e trade-offs tecnicos".to_string(),
            },
            PersonaInteraction {
                target_persona: "UX".to_string(),
                collaboration_contract: "Viabilizar experiencia".to_string(),
                expected_handoff: "Limites e oportunidades tecnicas".to_string(),
            },
            PersonaInteraction {
                target_persona: "DevOps".to_string(),
                collaboration_contract: "Preparar pipeline e rollout".to_string(),
                expected_handoff: "Artefatos para deploy e monitoracao".to_string(),
            },
        ],
        quality_criteria: vec![
            "Sem regressao funcional".to_string(),
            "Cobertura de teste adequada".to_string(),
        ],
    }
}

fn ux_persona() -> Persona {
    Persona {
        name: "UX".to_string(),
        purpose: "Garantir clareza de experiencia e usabilidade".to_string(),
        responsibilities: vec![
            "Desenhar fluxos e interfaces".to_string(),
            "Validar consistencia de interacao".to_string(),
        ],
        deliverables: vec![
            "Especificacao de interacao".to_string(),
            "Checklist de usabilidade".to_string(),
        ],
        operational_instructions: vec![
            "Antecipar friccoes no fluxo de usuario".to_string(),
            "Sincronizar decisoes visuais com Product".to_string(),
        ],
        interaction_matrix: vec![
            PersonaInteraction {
                target_persona: "Product".to_string(),
                collaboration_contract: "Refinar proposta de valor".to_string(),
                expected_handoff: "Cenarios de uso priorizados".to_string(),
            },
            PersonaInteraction {
                target_persona: "Engineering".to_string(),
                collaboration_contract: "Detalhar comportamento de interface".to_string(),
                expected_handoff: "Especificacoes de interacao implementaveis".to_string(),
            },
            PersonaInteraction {
                target_persona: "DevOps".to_string(),
                collaboration_contract: "Apoiar rollout progressivo".to_string(),
                expected_handoff: "Sinais de friccao para monitoracao".to_string(),
            },
        ],
        quality_criteria: vec![
            "Fluxo principal intuitivo".to_string(),
            "Consistencia de experiencia".to_string(),
        ],
    }
}

fn devops_persona() -> Persona {
    Persona {
        name: "DevOps".to_string(),
        purpose: "Assegurar entrega continua confiavel".to_string(),
        responsibilities: vec![
            "Automatizar build e release".to_string(),
            "Monitorar saude operacional".to_string(),
        ],
        deliverables: vec![
            "Pipeline validado".to_string(),
            "Plano de observabilidade e rollback".to_string(),
        ],
        operational_instructions: vec![
            "Reduzir risco de deploy com estrategia gradual".to_string(),
            "Garantir rastreabilidade de incidentes".to_string(),
        ],
        interaction_matrix: vec![
            PersonaInteraction {
                target_persona: "Product".to_string(),
                collaboration_contract: "Planejar janelas e impacto de release".to_string(),
                expected_handoff: "Status de rollout e risco".to_string(),
            },
            PersonaInteraction {
                target_persona: "Engineering".to_string(),
                collaboration_contract: "Padronizar operacao de deploy".to_string(),
                expected_handoff: "Requisitos de infraestrutura aplicados".to_string(),
            },
            PersonaInteraction {
                target_persona: "UX".to_string(),
                collaboration_contract: "Monitorar friccao de experiencia".to_string(),
                expected_handoff: "Metricas de uso em producao".to_string(),
            },
        ],
        quality_criteria: vec![
            "Deploy reproduzivel".to_string(),
            "Observabilidade acionavel".to_string(),
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
                field: "matriz_de_interacao"
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
                target_persona: "Inexistente".to_string(),
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
            if persona == "Product" && target == "Inexistente"
        ));
    }
}
