use std::fs;
use std::path::PathBuf;

use thiserror::Error;

pub const MAESTRO_DIR: &str = "maestro";
pub const SCOPES_DIR: &str = "scopes";
pub const PERSONAS_DIR: &str = "personas";
pub const SKILLS_DIR: &str = "skills";

#[derive(Debug, Error)]
pub enum MarkdownGovernanceError {
    #[error("Invalid scope file name: {0}. Use pattern 001-delivery-name.md")]
    InvalidScopeFileName(String),
    #[error("Scope number out of sequence. Expected: {expected:03}, found: {found:03}")]
    ScopeNumberOutOfSequence { expected: u16, found: u16 },
    #[error("Incomplete {document_type} document: missing required field: {field}")]
    MissingRequiredField {
        document_type: &'static str,
        field: &'static str,
    },
    #[error("Invalid markdown file name: {0}")]
    InvalidMarkdownFileName(String),
    #[error("Invalid persona name: {0}")]
    InvalidPersonaName(String),
    #[error("I/O error in markdown governance")]
    Io(#[from] std::io::Error),
}

pub struct MarkdownGovernance {
    root: PathBuf,
}

impl MarkdownGovernance {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn ensure_directories(&self) -> Result<(), MarkdownGovernanceError> {
        fs::create_dir_all(self.scopes_dir())?;
        fs::create_dir_all(self.personas_dir())?;
        fs::create_dir_all(self.skills_dir())?;
        Ok(())
    }

    pub fn validate_scope_document(
        &self,
        file_name: &str,
        content: &str,
    ) -> Result<PathBuf, MarkdownGovernanceError> {
        validate_scope_file_name(file_name)?;
        self.validate_scope_sequence(file_name)?;
        validate_required_fields(
            "scope",
            content,
            &[
                &["objective", "purpose", "objetivo"],
                &[
                    "business scope",
                    "scope",
                    "escopo de negocio",
                    "escopo de negócio",
                ],
                &["deliverables", "outputs", "entregaveis", "entregáveis"],
                &[
                    "acceptance criteria",
                    "criteria",
                    "criterios de aceite",
                    "critérios de aceite",
                ],
                &[
                    "dependencies",
                    "dependency map",
                    "dependencias",
                    "dependências",
                ],
            ],
            &[
                "objective",
                "business scope",
                "deliverables",
                "acceptance criteria",
                "dependencies",
            ],
        )?;

        Ok(self.scopes_dir().join(file_name))
    }

    pub fn validate_persona_document(
        &self,
        persona_file_name: &str,
        content: &str,
    ) -> Result<PathBuf, MarkdownGovernanceError> {
        validate_markdown_file_name(persona_file_name)?;
        validate_required_fields(
            "persona",
            content,
            &[
                &["responsibility", "responsibilities", "responsabilidade"],
                &["deliverables", "outputs", "entregaveis", "entregáveis"],
                &[
                    "operational instructions",
                    "instructions",
                    "instrucoes",
                    "instruções",
                ],
                &[
                    "interaction matrix",
                    "collaboration matrix",
                    "matriz de interacao",
                    "matriz de interação",
                ],
                &["boundaries", "limits", "limites"],
            ],
            &[
                "responsibility",
                "deliverables",
                "operational instructions",
                "interaction matrix",
                "boundaries",
            ],
        )?;

        Ok(self.personas_dir().join(persona_file_name))
    }

    pub fn validate_skill_document(
        &self,
        persona_name: &str,
        skill_file_name: &str,
        content: &str,
    ) -> Result<PathBuf, MarkdownGovernanceError> {
        validate_persona_name(persona_name)?;
        validate_markdown_file_name(skill_file_name)?;
        validate_required_fields(
            "skill",
            content,
            &[
                &["objective", "purpose", "objetivo"],
                &["triggers", "gatilhos"],
                &["inputs", "entradas"],
                &["outputs", "saidas", "saídas"],
                &["constraints", "restricoes", "restrições"],
            ],
            &["objective", "triggers", "inputs", "outputs", "constraints"],
        )?;

        Ok(self.skills_dir().join(persona_name).join(skill_file_name))
    }

    pub fn validate_scope_sequence(&self, file_name: &str) -> Result<(), MarkdownGovernanceError> {
        let found = parse_scope_number(file_name)
            .ok_or_else(|| MarkdownGovernanceError::InvalidScopeFileName(file_name.to_string()))?;

        let mut max_seen = 0_u16;
        let scopes_dir = self.scopes_dir();

        if scopes_dir.exists() {
            for entry in fs::read_dir(scopes_dir)? {
                let dir_entry = entry?;
                if let Some(name) = dir_entry.file_name().to_str() {
                    if let Some(value) = parse_scope_number(name) {
                        if value > max_seen {
                            max_seen = value;
                        }
                    }
                }
            }
        }

        let expected = max_seen.saturating_add(1);
        if found != expected {
            return Err(MarkdownGovernanceError::ScopeNumberOutOfSequence { expected, found });
        }

        Ok(())
    }

    pub fn scopes_dir(&self) -> PathBuf {
        self.root.join(MAESTRO_DIR).join(SCOPES_DIR)
    }

    pub fn personas_dir(&self) -> PathBuf {
        self.root.join(MAESTRO_DIR).join(PERSONAS_DIR)
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.root.join(MAESTRO_DIR).join(SKILLS_DIR)
    }
}

fn validate_scope_file_name(file_name: &str) -> Result<(), MarkdownGovernanceError> {
    validate_markdown_file_name(file_name)?;

    if parse_scope_number(file_name).is_none() {
        return Err(MarkdownGovernanceError::InvalidScopeFileName(
            file_name.to_string(),
        ));
    }

    Ok(())
}

fn validate_markdown_file_name(file_name: &str) -> Result<(), MarkdownGovernanceError> {
    if !file_name.ends_with(".md") {
        return Err(MarkdownGovernanceError::InvalidMarkdownFileName(
            file_name.to_string(),
        ));
    }

    let stem = file_name.trim_end_matches(".md");
    if stem.is_empty() || stem.contains('/') || stem.contains('\\') {
        return Err(MarkdownGovernanceError::InvalidMarkdownFileName(
            file_name.to_string(),
        ));
    }

    Ok(())
}

fn validate_persona_name(persona_name: &str) -> Result<(), MarkdownGovernanceError> {
    let trimmed = persona_name.trim();
    if trimmed.is_empty() || trimmed.contains('/') || trimmed.contains('\\') {
        return Err(MarkdownGovernanceError::InvalidPersonaName(
            persona_name.to_string(),
        ));
    }

    Ok(())
}

fn parse_scope_number(file_name: &str) -> Option<u16> {
    let stem = file_name.strip_suffix(".md")?;
    let (prefix, title) = stem.split_once('-')?;

    if prefix.len() != 3 || !prefix.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    if title.trim().is_empty() {
        return None;
    }

    prefix.parse::<u16>().ok()
}

fn validate_required_fields(
    document_type: &'static str,
    content: &str,
    aliases: &[&[&'static str]],
    canonical_fields: &[&'static str],
) -> Result<(), MarkdownGovernanceError> {
    let normalized = normalize(content);

    for (idx, alias_group) in aliases.iter().enumerate() {
        let found = alias_group.iter().any(|alias| normalized.contains(alias));
        if !found {
            return Err(MarkdownGovernanceError::MissingRequiredField {
                document_type,
                field: canonical_fields[idx],
            });
        }
    }

    Ok(())
}

fn normalize(input: &str) -> String {
    input.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn unique_root() -> PathBuf {
        std::env::temp_dir().join(format!("maestro-md-{}", Uuid::new_v4()))
    }

    fn scope_content() -> &'static str {
        "## Objective\ntext\n## Business Scope\ntext\n## Deliverables\ntext\n## Acceptance Criteria\ntext\n## Dependencies\ntext"
    }

    fn persona_content() -> &'static str {
        "## Responsibility\ntext\n## Deliverables\ntext\n## Operational Instructions\ntext\n## Interaction Matrix\ntext\n## Boundaries\ntext"
    }

    fn skill_content() -> &'static str {
        "## Objective\ntext\n## Triggers\ntext\n## Inputs\ntext\n## Outputs\ntext\n## Constraints\ntext"
    }

    #[test]
    fn ensures_governance_directories() {
        let root = unique_root();
        let governance = MarkdownGovernance::new(&root);

        let ensured = governance.ensure_directories();

        assert!(ensured.is_ok());
        assert!(governance.scopes_dir().exists());
        assert!(governance.personas_dir().exists());
        assert!(governance.skills_dir().exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_valid_scope_document_with_next_sequence() {
        let root = unique_root();
        let governance = MarkdownGovernance::new(&root);
        let ensured = governance.ensure_directories();
        assert!(ensured.is_ok());

        let first_scope = governance.validate_scope_document("001-Base.md", scope_content());
        assert!(first_scope.is_ok());

        let first_path = first_scope.ok();
        assert!(first_path.is_some());
        if let Some(path) = first_path {
            let write = fs::write(path, scope_content());
            assert!(write.is_ok());
        }

        let second_scope = governance.validate_scope_document("002-Runtime.md", scope_content());
        assert!(second_scope.is_ok());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_scope_out_of_sequence() {
        let root = unique_root();
        let governance = MarkdownGovernance::new(&root);
        let ensured = governance.ensure_directories();
        assert!(ensured.is_ok());

        let res = governance.validate_scope_document("002-Runtime.md", scope_content());

        assert!(matches!(
            res,
            Err(MarkdownGovernanceError::ScopeNumberOutOfSequence {
                expected: 1,
                found: 2
            })
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_scope_with_missing_required_fields() {
        let root = unique_root();
        let governance = MarkdownGovernance::new(&root);
        let ensured = governance.ensure_directories();
        assert!(ensured.is_ok());

        let content = "## Objective\ntext";
        let res = governance.validate_scope_document("001-Base.md", content);

        assert!(matches!(
            res,
            Err(MarkdownGovernanceError::MissingRequiredField {
                document_type: "scope",
                field: "business scope"
            })
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_valid_persona_document() {
        let root = unique_root();
        let governance = MarkdownGovernance::new(&root);

        let res = governance.validate_persona_document("produto.md", persona_content());

        assert!(res.is_ok());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_persona_document_without_matrix() {
        let root = unique_root();
        let governance = MarkdownGovernance::new(&root);
        let content =
            "## Responsibility\ntext\n## Deliverables\ntext\n## Operational Instructions\ntext\n## Boundaries\ntext";

        let res = governance.validate_persona_document("produto.md", content);

        assert!(matches!(
            res,
            Err(MarkdownGovernanceError::MissingRequiredField {
                document_type: "persona",
                field: "interaction matrix"
            })
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_valid_skill_document_path_under_persona_folder() {
        let root = unique_root();
        let governance = MarkdownGovernance::new(&root);

        let res =
            governance.validate_skill_document("engenharia", "code-review.md", skill_content());

        assert!(res.is_ok());
        let path = res.ok();
        assert!(path.is_some());
        if let Some(p) = path {
            let expected_prefix = governance.skills_dir().join("engenharia");
            assert!(p.starts_with(expected_prefix));
        }

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_skill_document_missing_required_fields() {
        let root = unique_root();
        let governance = MarkdownGovernance::new(&root);

        let res = governance.validate_skill_document(
            "engenharia",
            "code-review.md",
            "## Objective\ntext",
        );

        assert!(matches!(
            res,
            Err(MarkdownGovernanceError::MissingRequiredField {
                document_type: "skill",
                field: "triggers"
            })
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_legacy_portuguese_aliases_for_scope() {
        let root = unique_root();
        let governance = MarkdownGovernance::new(&root);
        let ensured = governance.ensure_directories();
        assert!(ensured.is_ok());

        let content = "## Objetivo\ntexto\n## Escopo de Negocio\ntexto\n## Entregaveis\ntexto\n## Criterios de Aceite\ntexto\n## Dependencias\ntexto";
        let res = governance.validate_scope_document("001-Base.md", content);

        assert!(res.is_ok());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_legacy_portuguese_aliases_for_persona() {
        let root = unique_root();
        let governance = MarkdownGovernance::new(&root);

        let content = "## Responsabilidade\ntexto\n## Entregaveis\ntexto\n## Instrucoes\ntexto\n## Matriz de Interacao\ntexto\n## Limites\ntexto";
        let res = governance.validate_persona_document("produto.md", content);

        assert!(res.is_ok());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn accepts_legacy_portuguese_aliases_for_skill() {
        let root = unique_root();
        let governance = MarkdownGovernance::new(&root);

        let content =
            "## Objetivo\ntexto\n## Gatilhos\ntexto\n## Entradas\ntexto\n## Saidas\ntexto\n## Restricoes\ntexto";
        let res = governance.validate_skill_document("engenharia", "code-review.md", content);

        assert!(res.is_ok());

        let _ = fs::remove_dir_all(root);
    }
}
