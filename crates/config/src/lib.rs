use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
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

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ToolsConfig {
    pub claude: ToolConfig,
    pub gemini: ToolConfig,
    pub codex: ToolConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ToolConfig {
    pub executable: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub display_name: String,
    pub repo_root: Option<PathBuf>,
    pub dev_logs_dir: PathBuf,
    pub adr_dir: PathBuf,
    pub transcript_dir: PathBuf,
    pub allowed_note_folders: Vec<PathBuf>,
    #[serde(default)]
    pub search_folders: Vec<PathBuf>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    vault_path: PathBuf,
    templates_dir: PathBuf,
    dev_log_template: PathBuf,
    adr_template: PathBuf,
    default_transcript_root: PathBuf,
    default_dev_log_root: PathBuf,
    default_adr_root: PathBuf,
    tools: ToolsConfig,
    projects: HashMap<String, ProjectConfig>,
    #[serde(default)]
    profiles: HashMap<String, ProfileConfig>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileConfig {
    vault_path: Option<PathBuf>,
    templates_dir: Option<PathBuf>,
    dev_log_template: Option<PathBuf>,
    adr_template: Option<PathBuf>,
    default_transcript_root: Option<PathBuf>,
    default_dev_log_root: Option<PathBuf>,
    default_adr_root: Option<PathBuf>,
    tools: Option<ToolsOverride>,
    projects: Option<HashMap<String, ProjectOverride>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolsOverride {
    claude: Option<ToolOverride>,
    gemini: Option<ToolOverride>,
    codex: Option<ToolOverride>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolOverride {
    executable: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectOverride {
    display_name: Option<String>,
    repo_root: Option<PathBuf>,
    dev_logs_dir: Option<PathBuf>,
    adr_dir: Option<PathBuf>,
    transcript_dir: Option<PathBuf>,
    allowed_note_folders: Option<Vec<PathBuf>>,
    search_folders: Option<Vec<PathBuf>>,
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
        Self::load_with_profile(path, None)
    }

    pub fn load_with_profile(path: impl AsRef<Path>, profile: Option<&str>) -> Result<Self> {
        let path = path.as_ref();
        let raw = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config at {}", path.display()))?;
        let parsed: RawConfig = toml::from_str(&raw)
            .with_context(|| format!("Failed to parse TOML config at {}", path.display()))?;
        let profile_name = profile
            .map(str::to_string)
            .or_else(|| std::env::var("AIW_PROFILE").ok());
        let mut config = Config {
            vault_path: parsed.vault_path,
            templates_dir: parsed.templates_dir,
            dev_log_template: parsed.dev_log_template,
            adr_template: parsed.adr_template,
            default_transcript_root: parsed.default_transcript_root,
            default_dev_log_root: parsed.default_dev_log_root,
            default_adr_root: parsed.default_adr_root,
            tools: parsed.tools,
            projects: parsed.projects,
        };
        if let Some(name) = profile_name.as_deref() {
            let selected = parsed
                .profiles
                .get(name)
                .ok_or_else(|| anyhow!("profile not found in config: {name}"))?;
            apply_profile(&mut config, selected)?;
        }
        apply_env_overrides(&mut config);
        Ok(config)
    }

    pub fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::default();

        if !self.vault_path.is_absolute() {
            report
                .errors
                .push("vault_path must be an absolute path".to_string());
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
            report
                .errors
                .push("project key cannot be empty".to_string());
        }
        if project.display_name.trim().is_empty() {
            report
                .errors
                .push(format!("project {key} display_name cannot be empty"));
        }

        if let Some(repo_root) = &project.repo_root {
            if !repo_root.is_absolute() {
                report
                    .errors
                    .push(format!("project {key} repo_root must be an absolute path"));
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
        self.validate_vault_relative(&format!("projects.{key}.adr_dir"), &project.adr_dir, report);
        self.validate_vault_relative(
            &format!("projects.{key}.transcript_dir"),
            &project.transcript_dir,
            report,
        );

        if project.allowed_note_folders.is_empty() {
            report.errors.push(format!(
                "project {key} allowed_note_folders cannot be empty"
            ));
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
            report.errors.push(format!(
                "tool {name} executable not found: {}",
                tool.executable
            ));
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
                report
                    .warnings
                    .push(format!("{field} does not exist yet: {}", path.display()));
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
            report
                .errors
                .push(format!("{field} must be a relative path"));
            return;
        }
        if has_parent_dir(path) {
            report
                .errors
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

fn apply_profile(config: &mut Config, profile: &ProfileConfig) -> Result<()> {
    if let Some(v) = &profile.vault_path {
        config.vault_path = v.clone();
    }
    if let Some(v) = &profile.templates_dir {
        config.templates_dir = v.clone();
    }
    if let Some(v) = &profile.dev_log_template {
        config.dev_log_template = v.clone();
    }
    if let Some(v) = &profile.adr_template {
        config.adr_template = v.clone();
    }
    if let Some(v) = &profile.default_transcript_root {
        config.default_transcript_root = v.clone();
    }
    if let Some(v) = &profile.default_dev_log_root {
        config.default_dev_log_root = v.clone();
    }
    if let Some(v) = &profile.default_adr_root {
        config.default_adr_root = v.clone();
    }

    if let Some(tools) = &profile.tools {
        if let Some(claude) = &tools.claude {
            if let Some(exec) = &claude.executable {
                config.tools.claude.executable = exec.clone();
            }
        }
        if let Some(gemini) = &tools.gemini {
            if let Some(exec) = &gemini.executable {
                config.tools.gemini.executable = exec.clone();
            }
        }
        if let Some(codex) = &tools.codex {
            if let Some(exec) = &codex.executable {
                config.tools.codex.executable = exec.clone();
            }
        }
    }

    if let Some(projects) = &profile.projects {
        for (key, override_cfg) in projects {
            let project = config
                .projects
                .get_mut(key)
                .ok_or_else(|| anyhow!("profile override references unknown project: {key}"))?;
            if let Some(v) = &override_cfg.display_name {
                project.display_name = v.clone();
            }
            if let Some(v) = &override_cfg.repo_root {
                project.repo_root = Some(v.clone());
            }
            if let Some(v) = &override_cfg.dev_logs_dir {
                project.dev_logs_dir = v.clone();
            }
            if let Some(v) = &override_cfg.adr_dir {
                project.adr_dir = v.clone();
            }
            if let Some(v) = &override_cfg.transcript_dir {
                project.transcript_dir = v.clone();
            }
            if let Some(v) = &override_cfg.allowed_note_folders {
                project.allowed_note_folders = v.clone();
            }
            if let Some(v) = &override_cfg.search_folders {
                project.search_folders = v.clone();
            }
        }
    }
    Ok(())
}

fn apply_env_overrides(config: &mut Config) {
    if let Ok(v) = std::env::var("AIW_VAULT_PATH") {
        config.vault_path = PathBuf::from(v);
    }
    if let Ok(v) = std::env::var("AIW_TEMPLATES_DIR") {
        config.templates_dir = PathBuf::from(v);
    }
    if let Ok(v) = std::env::var("AIW_DEV_LOG_TEMPLATE") {
        config.dev_log_template = PathBuf::from(v);
    }
    if let Ok(v) = std::env::var("AIW_ADR_TEMPLATE") {
        config.adr_template = PathBuf::from(v);
    }
    if let Ok(v) = std::env::var("AIW_DEFAULT_TRANSCRIPT_ROOT") {
        config.default_transcript_root = PathBuf::from(v);
    }
    if let Ok(v) = std::env::var("AIW_DEFAULT_DEV_LOG_ROOT") {
        config.default_dev_log_root = PathBuf::from(v);
    }
    if let Ok(v) = std::env::var("AIW_DEFAULT_ADR_ROOT") {
        config.default_adr_root = PathBuf::from(v);
    }
    if let Ok(v) = std::env::var("AIW_TOOL_CLAUDE_EXECUTABLE") {
        config.tools.claude.executable = v;
    }
    if let Ok(v) = std::env::var("AIW_TOOL_GEMINI_EXECUTABLE") {
        config.tools.gemini.executable = v;
    }
    if let Ok(v) = std::env::var("AIW_TOOL_CODEX_EXECUTABLE") {
        config.tools.codex.executable = v;
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
        let extensions: Vec<_> = pathext.split(';').filter(|ext| !ext.is_empty()).collect();
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
