use std::fs;
use std::path::Path;
use std::path::PathBuf;

use thiserror::Error;

pub const MAESTRO_DIR: &str = "maestro";
pub const SCOPES_DIR: &str = "scopes";
pub const PERSONAS_DIR: &str = "personas";
pub const SKILLS_DIR: &str = "skills";
pub const ARCHIVE_DIR: &str = "archive";
pub const MAESTRO_PERSONA_FILE: &str = "maestro.md";

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
    #[error("Invalid document path: {0}")]
    InvalidDocumentPath(String),
    #[error("Immutable persona cannot be modified: {0}")]
    ImmutablePersona(String),
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
        fs::create_dir_all(self.archive_dir())?;
        Ok(())
    }

    pub fn list_scopes(&self) -> Result<Vec<String>, MarkdownGovernanceError> {
        self.list_markdown_file_names(&self.scopes_dir())
    }

    pub fn list_personas(&self) -> Result<Vec<String>, MarkdownGovernanceError> {
        self.list_markdown_file_names(&self.personas_dir())
    }

    pub fn list_skills(&self, persona_name: &str) -> Result<Vec<String>, MarkdownGovernanceError> {
        validate_persona_name(persona_name)?;
        self.list_markdown_file_names(&self.skills_dir().join(persona_name))
    }

    pub fn read_document(&self, path: &Path) -> Result<String, MarkdownGovernanceError> {
        let normalized = self.normalize_document_path(path)?;
        if normalized.extension().and_then(|ext| ext.to_str()) != Some("md") {
            return Err(MarkdownGovernanceError::InvalidDocumentPath(
                normalized.display().to_string(),
            ));
        }

        Ok(fs::read_to_string(normalized)?)
    }

    pub fn archive_document(&self, path: &Path) -> Result<PathBuf, MarkdownGovernanceError> {
        let normalized = self.normalize_document_path(path)?;
        if normalized.extension().and_then(|ext| ext.to_str()) != Some("md") {
            return Err(MarkdownGovernanceError::InvalidDocumentPath(
                normalized.display().to_string(),
            ));
        }

        self.reject_immutable_target(&normalized)?;

        let maestro_root = self.root.join(MAESTRO_DIR);
        let relative = normalized.strip_prefix(&maestro_root).map_err(|_| {
            MarkdownGovernanceError::InvalidDocumentPath(normalized.display().to_string())
        })?;

        let destination = self.archive_dir().join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&normalized, &destination)?;

        Ok(destination)
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
        if is_maestro_persona_file(persona_file_name) {
            return Err(MarkdownGovernanceError::ImmutablePersona(
                persona_file_name.to_string(),
            ));
        }
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
        if is_maestro_persona_name(persona_name) {
            return Err(MarkdownGovernanceError::ImmutablePersona(
                persona_name.to_string(),
            ));
        }
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

    pub fn archive_dir(&self) -> PathBuf {
        self.root.join(MAESTRO_DIR).join(ARCHIVE_DIR)
    }

    fn list_markdown_file_names(
        &self,
        directory: &Path,
    ) -> Result<Vec<String>, MarkdownGovernanceError> {
        if !directory.exists() {
            return Ok(Vec::new());
        }

        let mut files = Vec::new();
        for entry in fs::read_dir(directory)? {
            let dir_entry = entry?;
            let path = dir_entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                files.push(name.to_string());
            }
        }

        files.sort();
        Ok(files)
    }

    fn normalize_document_path(&self, path: &Path) -> Result<PathBuf, MarkdownGovernanceError> {
        let normalized = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        };

        if !normalized.starts_with(self.root.join(MAESTRO_DIR)) {
            return Err(MarkdownGovernanceError::InvalidDocumentPath(
                path.display().to_string(),
            ));
        }

        Ok(normalized)
    }

    fn reject_immutable_target(&self, path: &Path) -> Result<(), MarkdownGovernanceError> {
        let maestro_root = self.root.join(MAESTRO_DIR);
        let relative = path.strip_prefix(&maestro_root).map_err(|_| {
            MarkdownGovernanceError::InvalidDocumentPath(path.display().to_string())
        })?;

        let mut components = relative.components();
        let first = components
            .next()
            .and_then(|c| c.as_os_str().to_str())
            .unwrap_or_default();

        if first == PERSONAS_DIR {
            if let Some(file_name) = components.next().and_then(|c| c.as_os_str().to_str()) {
                if is_maestro_persona_file(file_name) {
                    return Err(MarkdownGovernanceError::ImmutablePersona(
                        file_name.to_string(),
                    ));
                }
            }
        }

        if first == SKILLS_DIR {
            if let Some(persona_name) = components.next().and_then(|c| c.as_os_str().to_str()) {
                if is_maestro_persona_name(persona_name) {
                    return Err(MarkdownGovernanceError::ImmutablePersona(
                        persona_name.to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}

fn is_maestro_persona_name(persona_name: &str) -> bool {
    persona_name.trim().eq_ignore_ascii_case("maestro")
}

fn is_maestro_persona_file(file_name: &str) -> bool {
    file_name.trim().eq_ignore_ascii_case(MAESTRO_PERSONA_FILE)
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
        assert!(governance.archive_dir().exists());

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

    #[test]
    fn lists_personas_scopes_and_skills_sorted() {
        let root = unique_root();
        let governance = MarkdownGovernance::new(&root);
        assert!(governance.ensure_directories().is_ok());

        assert!(fs::write(governance.personas_dir().join("ux.md"), persona_content()).is_ok());
        assert!(fs::write(governance.personas_dir().join("qa.md"), persona_content()).is_ok());
        assert!(fs::write(governance.scopes_dir().join("002-beta.md"), scope_content()).is_ok());
        assert!(fs::write(
            governance.scopes_dir().join("001-alpha.md"),
            scope_content()
        )
        .is_ok());

        let skill_dir = governance.skills_dir().join("qa");
        assert!(fs::create_dir_all(&skill_dir).is_ok());
        assert!(fs::write(skill_dir.join("checklists.md"), skill_content()).is_ok());
        assert!(fs::write(skill_dir.join("automation.md"), skill_content()).is_ok());

        let personas = governance.list_personas();
        assert!(personas.is_ok());
        assert_eq!(personas.unwrap_or_default(), vec!["qa.md", "ux.md"]);

        let scopes = governance.list_scopes();
        assert!(scopes.is_ok());
        assert_eq!(
            scopes.unwrap_or_default(),
            vec!["001-alpha.md", "002-beta.md"]
        );

        let skills = governance.list_skills("qa");
        assert!(skills.is_ok());
        assert_eq!(
            skills.unwrap_or_default(),
            vec!["automation.md", "checklists.md"]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reads_document_under_maestro_tree() {
        let root = unique_root();
        let governance = MarkdownGovernance::new(&root);
        assert!(governance.ensure_directories().is_ok());

        let path = governance.personas_dir().join("qa.md");
        assert!(fs::write(&path, persona_content()).is_ok());

        let read = governance.read_document(&path);
        assert!(read.is_ok());
        assert!(read.unwrap_or_default().contains("## Responsibility"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn archives_non_immutable_document_to_archive_tree() {
        let root = unique_root();
        let governance = MarkdownGovernance::new(&root);
        assert!(governance.ensure_directories().is_ok());

        let path = governance.personas_dir().join("qa.md");
        assert!(fs::write(&path, persona_content()).is_ok());

        let archived = governance.archive_document(&path);
        assert!(archived.is_ok());
        let archived_path = archived.unwrap_or_else(|_| governance.archive_dir());
        assert!(!path.exists());
        assert!(archived_path.exists());
        assert!(archived_path.starts_with(governance.archive_dir().join(PERSONAS_DIR)));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_mutating_maestro_persona_and_skills() {
        let root = unique_root();
        let governance = MarkdownGovernance::new(&root);

        let persona_res = governance.validate_persona_document("maestro.md", persona_content());
        assert!(matches!(
            persona_res,
            Err(MarkdownGovernanceError::ImmutablePersona(_))
        ));

        let skill_res =
            governance.validate_skill_document("maestro", "routing.md", skill_content());
        assert!(matches!(
            skill_res,
            Err(MarkdownGovernanceError::ImmutablePersona(_))
        ));

        assert!(governance.ensure_directories().is_ok());
        let maestro_file = governance.personas_dir().join("maestro.md");
        assert!(fs::write(&maestro_file, persona_content()).is_ok());

        let archive_res = governance.archive_document(&maestro_file);
        assert!(matches!(
            archive_res,
            Err(MarkdownGovernanceError::ImmutablePersona(_))
        ));

        let _ = fs::remove_dir_all(root);
    }
}
