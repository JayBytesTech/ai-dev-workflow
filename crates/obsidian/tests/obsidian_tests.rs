use std::fs;
use std::path::PathBuf;

use aiw_config::{Config, ProjectConfig, ToolConfig, ToolsConfig};
use aiw_obsidian::{resolve_note_path, scan_content_for_commands, NoteCommand};

fn build_config(vault: &PathBuf) -> Config {
    let tool_path = vault.join("bin").join("tool");
    fs::create_dir_all(tool_path.parent().unwrap()).expect("tool dir");
    fs::write(&tool_path, "").expect("tool file");
    let tool = ToolConfig {
        executable: tool_path.to_string_lossy().to_string(),
    };

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
fn scan_content_detects_commands() {
    let content = "\n/ai summarize\ntext\n/ai extract-tasks\n";
    let matches = scan_content_for_commands(content);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].command, NoteCommand::Summarize);
    assert_eq!(matches[1].command, NoteCommand::ExtractTasks);
}

#[test]
fn resolve_note_path_respects_allowed_folders() {
    let dir = tempfile::tempdir().expect("tempdir");
    let vault = dir.path().to_path_buf();
    fs::create_dir_all(vault.join("Projects/AI Hub")).expect("projects");
    fs::create_dir_all(vault.join("Knowledge")).expect("knowledge");

    let config = build_config(&vault);
    let project = config.projects.get("ai-hub").expect("project");

    let ok_path = PathBuf::from("Projects/AI Hub/Notes.md");
    let resolved = resolve_note_path(&config, project, &ok_path).expect("resolve ok");
    assert!(resolved.starts_with(&vault));

    let bad_path = PathBuf::from("Knowledge/Outside.md");
    let result = resolve_note_path(&config, project, &bad_path);
    assert!(result.is_err());
}
