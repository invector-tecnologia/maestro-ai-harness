use super::*;

/// Directive families presented in Architect Mode, grouped for the picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DirectiveGroup {
    Personas,
    Skills,
    Scopes,
}

impl DirectiveGroup {
    pub(super) fn title(&self) -> &'static str {
        match self {
            DirectiveGroup::Personas => "Personas",
            DirectiveGroup::Skills => "Skills",
            DirectiveGroup::Scopes => "Scopes",
        }
    }
}

/// A selectable directive entry in the Architect Mode picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ArchitectEntry {
    pub(super) group: DirectiveGroup,
    pub(super) label: String,
    pub(super) file_name: String,
    pub(super) persona: Option<String>,
    pub(super) read_only: bool,
}

impl ArchitectEntry {
    /// Resolve the interview directive target this entry represents.
    pub(super) fn directive_target(&self) -> crate::application::interview_bot::DirectiveTarget {
        use crate::application::interview_bot::DirectiveTarget;
        match (&self.group, &self.persona) {
            (DirectiveGroup::Personas, _) => DirectiveTarget::Persona {
                name: directive_stem(&self.file_name),
            },
            (DirectiveGroup::Skills, Some(persona)) => DirectiveTarget::Skill {
                persona: persona.clone(),
                name: directive_stem(&self.file_name),
            },
            (DirectiveGroup::Skills, None) => DirectiveTarget::Skill {
                persona: String::new(),
                name: directive_stem(&self.file_name),
            },
            (DirectiveGroup::Scopes, _) => DirectiveTarget::Scope {
                name: directive_stem(&self.file_name),
            },
        }
    }
}

/// Interactive directives hub state for Architect Mode.
#[derive(Debug, Clone, Default)]
pub(super) struct ArchitectPicker {
    pub(super) entries: Vec<ArchitectEntry>,
    pub(super) cursor: usize,
}

impl ArchitectPicker {
    /// Build the picker from on-disk governance state, grouped by directive type.
    ///
    /// Maestro persona and its skills are listed but flagged read-only so they
    /// cannot be selected for mutation.
    pub(super) fn from_governance(governance: &MarkdownGovernance) -> Self {
        let mut entries = Vec::new();

        let personas = governance.list_personas().unwrap_or_default();
        for file_name in &personas {
            let read_only = is_maestro_directive_file(file_name);
            entries.push(ArchitectEntry {
                group: DirectiveGroup::Personas,
                label: directive_stem(file_name),
                file_name: file_name.clone(),
                persona: None,
                read_only,
            });
        }

        for persona_file in &personas {
            let persona_key = directive_stem(persona_file);
            let read_only = is_maestro_directive_name(&persona_key);
            let skills = governance.list_skills(&persona_key).unwrap_or_default();
            for file_name in skills {
                entries.push(ArchitectEntry {
                    group: DirectiveGroup::Skills,
                    label: format!("{}/{}", persona_key, directive_stem(&file_name)),
                    file_name,
                    persona: Some(persona_key.clone()),
                    read_only,
                });
            }
        }

        for file_name in governance.list_scopes().unwrap_or_default() {
            entries.push(ArchitectEntry {
                group: DirectiveGroup::Scopes,
                label: directive_stem(&file_name),
                file_name,
                persona: None,
                read_only: false,
            });
        }

        Self { entries, cursor: 0 }
    }

    pub(super) fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub(super) fn move_down(&mut self) {
        if self.cursor + 1 < self.entries.len() {
            self.cursor += 1;
        }
    }

    pub(super) fn selected(&self) -> Option<&ArchitectEntry> {
        self.entries.get(self.cursor)
    }
}

fn directive_stem(file_name: &str) -> String {
    file_name
        .strip_suffix(".md")
        .unwrap_or(file_name)
        .to_string()
}

fn is_maestro_directive_file(file_name: &str) -> bool {
    file_name.eq_ignore_ascii_case(MAESTRO_PERSONA_FILE)
}

fn is_maestro_directive_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("maestro")
}
