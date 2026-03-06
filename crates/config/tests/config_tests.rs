use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use aiw_config::{resolve_in_vault, Config, ProjectConfig, ToolConfig, ToolsConfig};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn temp_tool(path: &Path) -> ToolConfig {
    fs::write(path, "").expect("create tool file");
    ToolConfig {
        executable: path.to_string_lossy().to_string(),
    }
}

fn minimal_config(vault: &Path) -> Config {
    let tool_path = vault.join("bin").join("tool");
    fs::create_dir_all(tool_path.parent().unwrap()).expect("tool dir");
    let tool = temp_tool(&tool_path);

    let project = ProjectConfig {
        display_name: "AI Hub".to_string(),
        repo_root: None,
        dev_logs_dir: PathBuf::from("Dev Logs/AI Hub"),
        adr_dir: PathBuf::from("ADR/AI Hub"),
        transcript_dir: PathBuf::from("AI Sessions/raw/AI Hub"),
        allowed_note_folders: vec![PathBuf::from("Projects/AI Hub")],
    };

    let mut projects = std::collections::HashMap::new();
    projects.insert("ai-hub".to_string(), project);

    Config {
        vault_path: vault.to_path_buf(),
        templates_dir: PathBuf::from("Templates"),
        dev_log_template: PathBuf::from("AIW_Dev_Log.md"),
        adr_template: PathBuf::from("AIW_ADR.md"),
        default_transcript_root: PathBuf::from("AI Sessions/raw"),
        default_dev_log_root: PathBuf::from("Dev Logs"),
        default_adr_root: PathBuf::from("ADR"),
        tools: ToolsConfig {
            claude: tool.clone(),
            gemini: tool.clone(),
            codex: tool,
        },
        projects,
    }
}

#[test]
fn resolve_in_vault_relative() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = dir.path().to_path_buf();
    let resolved = resolve_in_vault(&vault, &PathBuf::from("Projects/AI Hub")).expect("resolve");
    assert!(resolved.starts_with(&vault));
}

#[test]
fn resolve_in_vault_absolute_ok() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = dir.path().to_path_buf();
    let absolute = vault.join("Knowledge");
    let resolved = resolve_in_vault(&vault, &absolute).expect("resolve");
    assert_eq!(resolved, absolute);
}

#[test]
fn resolve_in_vault_rejects_outside() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = dir.path().to_path_buf();
    let other = dir.path().parent().expect("parent").join("outside");
    let result = resolve_in_vault(&vault, &other);
    assert!(result.is_err());
}

#[test]
fn validate_reports_duplicate_allowed_folders() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = dir.path().to_path_buf();
    fs::create_dir_all(vault.join("Templates")).expect("templates");
    fs::create_dir_all(vault.join("Projects/AI Hub")).expect("projects");

    let mut config = minimal_config(&vault);
    if let Some(project) = config.projects.get_mut("ai-hub") {
        project
            .allowed_note_folders
            .push(PathBuf::from("Projects/AI Hub"));
    }

    let report = config.validate();
    assert!(!report.warnings.is_empty());
    let found = report
        .warnings
        .iter()
        .any(|w| w.contains("duplicate entry"));
    assert!(found);
}

#[test]
fn validate_ok_for_minimal_config() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = dir.path().to_path_buf();
    fs::create_dir_all(vault.join("Templates")).expect("templates");
    fs::create_dir_all(vault.join("Projects/AI Hub")).expect("projects");

    let config = minimal_config(&vault);
    let report = config.validate();
    assert!(report.errors.is_empty());
}

#[test]
fn load_with_profile_applies_overrides() {
    let _guard = ENV_LOCK.lock().expect("lock");
    // SAFETY: clear process env keys touched by other config tests.
    unsafe {
        std::env::remove_var("AIW_VAULT_PATH");
        std::env::remove_var("AIW_TOOL_CODEX_EXECUTABLE");
        std::env::remove_var("AIW_PROFILE");
    }
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = dir.path().join("vault");
    let ci_vault = dir.path().join("vault-ci");
    fs::create_dir_all(vault.join("Templates")).expect("templates");
    fs::create_dir_all(ci_vault.join("Templates")).expect("ci templates");

    let tool = dir.path().join("bin").join("codex-ci");
    fs::create_dir_all(tool.parent().expect("tool parent")).expect("tool parent mkdir");
    fs::write(&tool, "").expect("tool file");

    let config_path = dir.path().join("ai-dev-workflow.toml");
    let toml = format!(
        "vault_path = \"{}\"\n\
templates_dir = \"Templates\"\n\
dev_log_template = \"AIW_Dev_Log.md\"\n\
adr_template = \"AIW_ADR.md\"\n\
default_transcript_root = \"AI Sessions/raw\"\n\
default_dev_log_root = \"Dev Logs\"\n\
default_adr_root = \"ADR\"\n\
\n\
[tools.claude]\n\
executable = \"{}\"\n\
[tools.gemini]\n\
executable = \"{}\"\n\
[tools.codex]\n\
executable = \"{}\"\n\
\n\
[projects.ai-hub]\n\
display_name = \"AI Hub\"\n\
repo_root = \"{}\"\n\
dev_logs_dir = \"Dev Logs/AI Hub\"\n\
adr_dir = \"ADR/AI Hub\"\n\
transcript_dir = \"AI Sessions/raw/AI Hub\"\n\
allowed_note_folders = [\"Projects/AI Hub\"]\n\
\n\
[profiles.ci]\n\
vault_path = \"{}\"\n\
\n\
[profiles.ci.tools.codex]\n\
executable = \"{}\"\n\
\n\
[profiles.ci.projects.ai-hub]\n\
transcript_dir = \"AI Sessions/raw/AI Hub/CI\"\n",
        vault.display(),
        tool.display(),
        tool.display(),
        tool.display(),
        dir.path().display(),
        ci_vault.display(),
        tool.display(),
    );
    fs::write(&config_path, toml).expect("write config");

    let cfg = Config::load_with_profile(&config_path, Some("ci")).expect("load ci profile");
    assert_eq!(cfg.vault_path, ci_vault);
    assert_eq!(cfg.tools.codex.executable, tool.to_string_lossy());
    assert_eq!(
        cfg.projects
            .get("ai-hub")
            .expect("project")
            .transcript_dir
            .to_string_lossy(),
        "AI Sessions/raw/AI Hub/CI"
    );
}

#[test]
fn load_with_profile_applies_env_overrides() {
    let _guard = ENV_LOCK.lock().expect("lock");
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = dir.path().join("vault");
    fs::create_dir_all(vault.join("Templates")).expect("templates");

    let tool = dir.path().join("bin").join("tool");
    fs::create_dir_all(tool.parent().expect("tool parent")).expect("tool parent mkdir");
    fs::write(&tool, "").expect("tool file");

    let config_path = dir.path().join("ai-dev-workflow.toml");
    let toml = format!(
        "vault_path = \"{}\"\n\
templates_dir = \"Templates\"\n\
dev_log_template = \"AIW_Dev_Log.md\"\n\
adr_template = \"AIW_ADR.md\"\n\
default_transcript_root = \"AI Sessions/raw\"\n\
default_dev_log_root = \"Dev Logs\"\n\
default_adr_root = \"ADR\"\n\
\n\
[tools.claude]\n\
executable = \"{}\"\n\
[tools.gemini]\n\
executable = \"{}\"\n\
[tools.codex]\n\
executable = \"{}\"\n\
\n\
[projects.ai-hub]\n\
display_name = \"AI Hub\"\n\
repo_root = \"{}\"\n\
dev_logs_dir = \"Dev Logs/AI Hub\"\n\
adr_dir = \"ADR/AI Hub\"\n\
transcript_dir = \"AI Sessions/raw/AI Hub\"\n\
allowed_note_folders = [\"Projects/AI Hub\"]\n",
        vault.display(),
        tool.display(),
        tool.display(),
        tool.display(),
        dir.path().display(),
    );
    fs::write(&config_path, toml).expect("write config");

    let override_path = dir.path().join("vault-override");
    fs::create_dir_all(&override_path).expect("override vault");
    // SAFETY: test process owns these vars for the duration of this test and removes them after use.
    unsafe {
        std::env::remove_var("AIW_PROFILE");
        std::env::set_var("AIW_VAULT_PATH", override_path.to_string_lossy().as_ref());
        std::env::set_var("AIW_TOOL_CODEX_EXECUTABLE", tool.to_string_lossy().as_ref());
    }
    let cfg = Config::load_with_profile(&config_path, None).expect("load with env override");
    // SAFETY: clean up test-only variables.
    unsafe {
        std::env::remove_var("AIW_VAULT_PATH");
        std::env::remove_var("AIW_TOOL_CODEX_EXECUTABLE");
    }

    assert_eq!(cfg.vault_path, override_path);
    assert_eq!(cfg.tools.codex.executable, tool.to_string_lossy());
}
