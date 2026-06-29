use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDepsConfig {
    #[serde(default)]
    pub dependencies: Vec<ProjectDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDependency {
    pub name: String,
    pub check_command: String,
    #[serde(default = "default_required")]
    pub required: bool,
    pub install_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectDependencyCheck {
    pub name: String,
    pub passed: bool,
    pub required: bool,
    pub install_hint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectDepsCheckReport {
    pub checks: Vec<ProjectDependencyCheck>,
}

impl ProjectDepsCheckReport {
    pub fn all_required_passed(&self) -> bool {
        self.checks.iter().all(|c| !c.required || c.passed)
    }
}

#[derive(Debug, Error)]
pub enum ProjectDepsError {
    #[error("Project deps file not found: {0}")]
    NotFound(String),
    #[error("Invalid YAML syntax: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Invalid project dependency config: {0}")]
    Validation(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl ProjectDepsConfig {
    pub fn load(path: Option<PathBuf>) -> Result<Self, ProjectDepsError> {
        let path = path.unwrap_or_else(default_project_deps_path);
        if !path.exists() {
            return Err(ProjectDepsError::NotFound(
                path.to_string_lossy().to_string(),
            ));
        }

        let content = fs::read_to_string(path)?;
        let config: ProjectDepsConfig = serde_yaml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ProjectDepsError> {
        for dep in &self.dependencies {
            if dep.name.trim().is_empty() {
                return Err(ProjectDepsError::Validation(
                    "dependency name cannot be empty".to_string(),
                ));
            }
            if dep.check_command.trim().is_empty() {
                return Err(ProjectDepsError::Validation(format!(
                    "dependency '{}' has empty check_command",
                    dep.name
                )));
            }
        }

        Ok(())
    }
}

pub fn default_project_deps_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("maestro")
        .join("project-deps.yml")
}

pub const DEFAULT_PROJECT_DEPS_TEMPLATE: &str = "dependencies:\n  - name: git\n    check_command: \"command -v git >/dev/null 2>&1\"\n    required: true\n    install_hint: \"Install Git and ensure it is available in PATH.\"\n  - name: cargo\n    check_command: \"command -v cargo >/dev/null 2>&1\"\n    required: false\n    install_hint: \"Install Rust toolchain if this project uses Rust.\"\n";

fn default_required() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_template() {
        let parsed: Result<ProjectDepsConfig, serde_yaml::Error> =
            serde_yaml::from_str(DEFAULT_PROJECT_DEPS_TEMPLATE);
        assert!(parsed.is_ok());

        let cfg = parsed.unwrap_or(ProjectDepsConfig {
            dependencies: vec![],
        });
        assert!(!cfg.dependencies.is_empty());
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn rejects_empty_check_command() {
        let cfg = ProjectDepsConfig {
            dependencies: vec![ProjectDependency {
                name: "git".to_string(),
                check_command: " ".to_string(),
                required: true,
                install_hint: None,
            }],
        };

        assert!(cfg.validate().is_err());
    }
}
