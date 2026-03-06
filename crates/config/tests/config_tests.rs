use std::fs;
use std::path::PathBuf;

use aiw_config::{resolve_in_vault, Config, ProjectConfig, ToolConfig, ToolsConfig};

fn temp_tool(path: &PathBuf) -> ToolConfig {
    fs::write(path, "").expect("create tool file");
    ToolConfig {
        executable: path.to_string_lossy().to_string(),
    }
}

fn minimal_config(vault: &PathBuf) -> Config {
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
        vault_path: vault.clone(),
        templates_dir: PathBuf::from("Templates"),
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
    let resolved = resolve_in_vault(&vault, &PathBuf::from("Projects/AI Hub"))
        .expect("resolve");
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
    let other = dir
        .path()
        .parent()
        .expect("parent")
        .join("outside");
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
        project.allowed_note_folders.push(PathBuf::from("Projects/AI Hub"));
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
