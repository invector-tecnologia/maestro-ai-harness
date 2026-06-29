use super::*;

#[derive(Debug, Clone)]
pub enum WizardSubmission {
    Persona {
        file_name: String,
        content: String,
    },
    Scope {
        file_name: String,
        content: String,
    },
    Skill {
        persona_name: String,
        file_name: String,
        content: String,
    },
    ArchivePersona {
        file_name: String,
    },
    ArchiveScope {
        file_name: String,
    },
    ArchiveSkill {
        persona_name: String,
        file_name: String,
    },
}

pub(super) enum WizardAdvance {
    NeedMoreInput,
    ValidationError(String),
    Completed(WizardSubmission),
}

#[derive(Debug, Clone)]
pub(super) enum WizardKind {
    Persona,
    Scope,
    Skill,
}

impl WizardKind {
    pub(super) fn label(&self) -> &'static str {
        match self {
            WizardKind::Persona => "persona",
            WizardKind::Scope => "scope",
            WizardKind::Skill => "skill",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct WizardField {
    prompt: &'static str,
    value: String,
}

#[derive(Debug, Clone)]
pub(super) struct CreationWizard {
    pub(super) kind: WizardKind,
    fields: Vec<WizardField>,
    cursor: usize,
}

impl CreationWizard {
    pub(super) fn new_persona() -> Self {
        Self {
            kind: WizardKind::Persona,
            fields: vec![
                WizardField {
                    prompt: "persona name",
                    value: String::new(),
                },
                WizardField {
                    prompt: "responsibility",
                    value: String::new(),
                },
                WizardField {
                    prompt: "deliverables",
                    value: String::new(),
                },
                WizardField {
                    prompt: "instructions",
                    value: String::new(),
                },
                WizardField {
                    prompt: "interaction matrix",
                    value: String::new(),
                },
                WizardField {
                    prompt: "limits",
                    value: String::new(),
                },
            ],
            cursor: 0,
        }
    }

    pub(super) fn new_scope() -> Self {
        Self {
            kind: WizardKind::Scope,
            fields: vec![
                WizardField {
                    prompt: "delivery number (e.g. 001)",
                    value: String::new(),
                },
                WizardField {
                    prompt: "delivery name",
                    value: String::new(),
                },
                WizardField {
                    prompt: "objective",
                    value: String::new(),
                },
                WizardField {
                    prompt: "business scope",
                    value: String::new(),
                },
                WizardField {
                    prompt: "deliverables",
                    value: String::new(),
                },
                WizardField {
                    prompt: "acceptance criteria",
                    value: String::new(),
                },
                WizardField {
                    prompt: "dependencies",
                    value: String::new(),
                },
            ],
            cursor: 0,
        }
    }

    pub(super) fn new_skill() -> Self {
        Self {
            kind: WizardKind::Skill,
            fields: vec![
                WizardField {
                    prompt: "target persona",
                    value: String::new(),
                },
                WizardField {
                    prompt: "skill name",
                    value: String::new(),
                },
                WizardField {
                    prompt: "objective",
                    value: String::new(),
                },
                WizardField {
                    prompt: "triggers",
                    value: String::new(),
                },
                WizardField {
                    prompt: "inputs",
                    value: String::new(),
                },
                WizardField {
                    prompt: "outputs",
                    value: String::new(),
                },
                WizardField {
                    prompt: "constraints",
                    value: String::new(),
                },
            ],
            cursor: 0,
        }
    }

    pub(super) fn current_prompt(&self) -> &str {
        self.fields
            .get(self.cursor)
            .map(|field| field.prompt)
            .unwrap_or("finish")
    }

    pub(super) fn advance(&mut self, raw_input: &str) -> WizardAdvance {
        if raw_input.trim().is_empty() {
            return WizardAdvance::ValidationError(format!(
                "required field: {}",
                self.current_prompt()
            ));
        }

        if let Some(field) = self.fields.get_mut(self.cursor) {
            field.value = raw_input.trim().to_string();
        }

        self.cursor += 1;
        if self.cursor < self.fields.len() {
            return WizardAdvance::NeedMoreInput;
        }

        match self.kind {
            WizardKind::Persona => WizardAdvance::Completed(self.to_persona_submission()),
            WizardKind::Scope => match self.to_scope_submission() {
                Ok(submission) => WizardAdvance::Completed(submission),
                Err(error) => {
                    self.cursor = 0;
                    WizardAdvance::ValidationError(error)
                }
            },
            WizardKind::Skill => WizardAdvance::Completed(self.to_skill_submission()),
        }
    }

    fn to_persona_submission(&self) -> WizardSubmission {
        let persona_name = self.fields[0].value.clone();
        let file_name = format!("{}.md", slug(&persona_name));
        let content = format!(
            "## Responsibility\n{}\n\n## Deliverables\n{}\n\n## Instructions\n{}\n\n## Interaction Matrix\n{}\n\n## Limits\n{}\n",
            self.fields[1].value,
            self.fields[2].value,
            self.fields[3].value,
            self.fields[4].value,
            self.fields[5].value,
        );

        WizardSubmission::Persona { file_name, content }
    }

    fn to_scope_submission(&self) -> Result<WizardSubmission, String> {
        let number = self.fields[0].value.trim();
        if number.len() != 3 || !number.chars().all(|ch| ch.is_ascii_digit()) {
            return Err("delivery number must have 3 digits (e.g. 001)".to_string());
        }

        let file_name = format!("{}-{}.md", number, slug(&self.fields[1].value));
        let content = format!(
            "## Objective\n{}\n\n## Business Scope\n{}\n\n## Deliverables\n{}\n\n## Acceptance Criteria\n{}\n\n## Dependencies\n{}\n",
            self.fields[2].value,
            self.fields[3].value,
            self.fields[4].value,
            self.fields[5].value,
            self.fields[6].value,
        );

        Ok(WizardSubmission::Scope { file_name, content })
    }

    fn to_skill_submission(&self) -> WizardSubmission {
        let persona_name = self.fields[0].value.clone();
        let file_name = format!("{}.md", slug(&self.fields[1].value));
        let content = format!(
            "## Objective\n{}\n\n## Triggers\n{}\n\n## Inputs\n{}\n\n## Outputs\n{}\n\n## Constraints\n{}\n",
            self.fields[2].value,
            self.fields[3].value,
            self.fields[4].value,
            self.fields[5].value,
            self.fields[6].value,
        );

        WizardSubmission::Skill {
            persona_name,
            file_name,
            content,
        }
    }
}

pub(super) fn persist_submission(
    governance: &MarkdownGovernance,
    submission: WizardSubmission,
) -> Result<PathBuf, anyhow::Error> {
    governance.ensure_directories()?;

    let path = match submission {
        WizardSubmission::Persona { file_name, content } => {
            let path = governance.validate_persona_document(&file_name, &content)?;
            std::fs::write(&path, content)?;
            path
        }
        WizardSubmission::Scope { file_name, content } => {
            let path = governance.validate_scope_document(&file_name, &content)?;
            std::fs::write(&path, content)?;
            path
        }
        WizardSubmission::Skill {
            persona_name,
            file_name,
            content,
        } => {
            let path = governance.validate_skill_document(&persona_name, &file_name, &content)?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, content)?;
            path
        }
        WizardSubmission::ArchivePersona { file_name } => {
            let path = governance.personas_dir().join(file_name);
            governance.archive_document(&path)?
        }
        WizardSubmission::ArchiveScope { file_name } => {
            let path = governance.scopes_dir().join(file_name);
            governance.archive_document(&path)?
        }
        WizardSubmission::ArchiveSkill {
            persona_name,
            file_name,
        } => {
            let path = governance.skills_dir().join(persona_name).join(file_name);
            governance.archive_document(&path)?
        }
    };

    Ok(path)
}

pub(super) fn slug(input: &str) -> String {
    let lowered = input.trim().to_lowercase();
    let mut out = String::new();
    let mut last_dash = false;

    for ch in lowered.chars() {
        let mapped = match ch {
            'a'..='z' | '0'..='9' => Some(ch),
            _ => Some('-'),
        };

        if let Some(value) = mapped {
            if value == '-' {
                if !last_dash {
                    out.push('-');
                    last_dash = true;
                }
            } else {
                out.push(value);
                last_dash = false;
            }
        }
    }

    out.trim_matches('-').to_string()
}
