use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub vault_path: PathBuf,
    pub templates_dir: PathBuf,
    pub dev_log_template: PathBuf,
    pub adr_template: PathBuf,
    pub default_transcript_root: PathBuf,
    pub default_dev_log_root: PathBuf,
    pub default_adr_root: PathBuf,
    pub tools: ToolsConfig,
    pub projects: HashMap<String, ProjectConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolsConfig {
    pub claude: ToolConfig,
    pub gemini: ToolConfig,
    pub codex: ToolConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ToolConfig {
    pub executable: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub display_name: String,
    pub repo_root: Option<PathBuf>,
    pub dev_logs_dir: PathBuf,
    pub adr_dir: PathBuf,
    pub transcript_dir: PathBuf,
    pub allowed_note_folders: Vec<PathBuf>,
}

#[derive(Debug, Default)]
pub struct ValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationReport {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

impl fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.errors.is_empty() && self.warnings.is_empty() {
            writeln!(f, "Config validation: OK")?;
            return Ok(());
        }
        if !self.errors.is_empty() {
            writeln!(f, "Config validation errors:")?;
            for err in &self.errors {
                writeln!(f, "- {err}")?;
            }
        }
        if !self.warnings.is_empty() {
            writeln!(f, "Config validation warnings:")?;
            for warn in &self.warnings {
                writeln!(f, "- {warn}")?;
            }
        }
        Ok(())
    }
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        let config: Config = toml::from_str(&raw)
            .with_context(|| format!("Failed to parse TOML config at {}", path.display()))?;
        Ok(config)
    }

    pub fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::default();

        if !self.vault_path.is_absolute() {
            report.errors.push("vault_path must be an absolute path".to_string());
        } else if !self.vault_path.exists() {
            report.errors.push(format!(
                "vault_path does not exist: {}",
                self.vault_path.display()
            ));
        } else if !self.vault_path.is_dir() {
            report.errors.push(format!(
                "vault_path is not a directory: {}",
                self.vault_path.display()
            ));
        }

        self.validate_vault_relative("templates_dir", &self.templates_dir, &mut report);
        self.validate_template_name("dev_log_template", &self.dev_log_template, &mut report);
        self.validate_template_name("adr_template", &self.adr_template, &mut report);
        self.validate_vault_relative(
            "default_transcript_root",
            &self.default_transcript_root,
            &mut report,
        );
        self.validate_vault_relative(
            "default_dev_log_root",
            &self.default_dev_log_root,
            &mut report,
        );
        self.validate_vault_relative("default_adr_root", &self.default_adr_root, &mut report);

        self.validate_tool("claude", &self.tools.claude, &mut report);
        self.validate_tool("gemini", &self.tools.gemini, &mut report);
        self.validate_tool("codex", &self.tools.codex, &mut report);

        if self.projects.is_empty() {
            report
                .errors
                .push("at least one project must be configured".to_string());
        }

        for (key, project) in &self.projects {
            self.validate_project(key, project, &mut report);
        }

        report
    }

    fn validate_project(&self, key: &str, project: &ProjectConfig, report: &mut ValidationReport) {
        if key.trim().is_empty() {
            report.errors.push("project key cannot be empty".to_string());
        }
        if project.display_name.trim().is_empty() {
            report.errors.push(format!("project {key} display_name cannot be empty"));
        }

        if let Some(repo_root) = &project.repo_root {
            if !repo_root.is_absolute() {
                report.errors.push(format!(
                    "project {key} repo_root must be an absolute path"
                ));
            } else if !repo_root.exists() {
                report.warnings.push(format!(
                    "project {key} repo_root does not exist: {}",
                    repo_root.display()
                ));
            } else if !repo_root.is_dir() {
                report.errors.push(format!(
                    "project {key} repo_root is not a directory: {}",
                    repo_root.display()
                ));
            }
        }

        self.validate_vault_relative(
            &format!("projects.{key}.dev_logs_dir"),
            &project.dev_logs_dir,
            report,
        );
        self.validate_vault_relative(
            &format!("projects.{key}.adr_dir"),
            &project.adr_dir,
            report,
        );
        self.validate_vault_relative(
            &format!("projects.{key}.transcript_dir"),
            &project.transcript_dir,
            report,
        );

        if project.allowed_note_folders.is_empty() {
            report
                .errors
                .push(format!("project {key} allowed_note_folders cannot be empty"));
        }

        let mut seen = HashSet::new();
        for folder in &project.allowed_note_folders {
            self.validate_vault_relative(
                &format!("projects.{key}.allowed_note_folders"),
                folder,
                report,
            );
            let normalized = folder.to_string_lossy().to_string();
            if !seen.insert(normalized.clone()) {
                report.warnings.push(format!(
                    "project {key} allowed_note_folders has duplicate entry: {normalized}"
                ));
            }
        }
    }

    fn validate_tool(&self, name: &str, tool: &ToolConfig, report: &mut ValidationReport) {
        if tool.executable.trim().is_empty() {
            report
                .errors
                .push(format!("tool {name} executable cannot be empty"));
            return;
        }
        if executable_exists(&tool.executable).is_err() {
            report
                .errors
                .push(format!("tool {name} executable not found: {}", tool.executable));
        }
    }

    fn validate_vault_relative(&self, field: &str, path: &Path, report: &mut ValidationReport) {
        if path.as_os_str().is_empty() {
            report.errors.push(format!("{field} cannot be empty"));
            return;
        }

        if has_parent_dir(path) {
            report
                .errors
                .push(format!("{field} cannot contain '..' segments"));
            return;
        }

        if path.is_absolute() {
            if !self.path_under_vault(path) {
                report.errors.push(format!(
                    "{field} must be within vault_path: {}",
                    path.display()
                ));
            }
            if !path.exists() {
                report.warnings.push(format!(
                    "{field} does not exist yet: {}",
                    path.display()
                ));
            }
            return;
        }

        let resolved = self.vault_path.join(path);
        if !resolved.exists() {
            report.warnings.push(format!(
                "{field} does not exist yet: {}",
                resolved.display()
            ));
        }
    }

    fn validate_template_name(&self, field: &str, path: &Path, report: &mut ValidationReport) {
        if path.as_os_str().is_empty() {
            report.errors.push(format!("{field} cannot be empty"));
            return;
        }
        if path.is_absolute() {
            report.errors.push(format!("{field} must be a relative path"));
            return;
        }
        if has_parent_dir(path) {
            report.errors
                .push(format!("{field} cannot contain '..' segments"));
            return;
        }

        let templates_root = self.vault_path.join(&self.templates_dir);
        let template_path = templates_root.join(path);
        if templates_root.exists() && !template_path.exists() {
            report.warnings.push(format!(
                "{field} does not exist yet: {}",
                template_path.display()
            ));
        }
    }

    fn path_under_vault(&self, path: &Path) -> bool {
        let vault = match self.vault_path.canonicalize() {
            Ok(path) => path,
            Err(_) => return false,
        };
        match path.canonicalize() {
            Ok(candidate) => candidate.starts_with(vault),
            Err(_) => false,
        }
    }
}

pub fn resolve_in_vault(vault: &Path, path: &Path) -> Result<PathBuf> {
    if has_parent_dir(path) {
        return Err(anyhow!("path cannot contain '..' segments"));
    }
    if path.is_absolute() {
        if !path.starts_with(vault) {
            return Err(anyhow!("path must be within vault_path"));
        }
        return Ok(path.to_path_buf());
    }
    Ok(vault.join(path))
}

fn has_parent_dir(path: &Path) -> bool {
    path.components().any(|c| matches!(c, Component::ParentDir))
}

fn executable_exists(executable: &str) -> Result<()> {
    let path = Path::new(executable);
    if path.components().count() > 1 {
        if path.exists() {
            return Ok(());
        }
        return Err(anyhow!("executable path not found"));
    }

    let path_var = std::env::var_os("PATH").ok_or_else(|| anyhow!("PATH not set"))?;
    let paths = std::env::split_paths(&path_var);

    if cfg!(windows) {
        let pathext = std::env::var_os("PATHEXT")
            .unwrap_or_else(|| OsStr::new(".EXE;.CMD;.BAT").to_os_string());
        let pathext = pathext.to_string_lossy().to_string();
        let extensions: Vec<_> = pathext
            .split(';')
            .filter(|ext| !ext.is_empty())
            .collect();
        for dir in paths {
            for ext in &extensions {
                let candidate = dir.join(format!("{}{}", executable, ext));
                if candidate.exists() {
                    return Ok(());
                }
            }
        }
        return Err(anyhow!("executable not found in PATH"));
    }

    for dir in paths {
        let candidate = dir.join(executable);
        if candidate.exists() {
            return Ok(());
        }
    }

    Err(anyhow!("executable not found in PATH"))
}
