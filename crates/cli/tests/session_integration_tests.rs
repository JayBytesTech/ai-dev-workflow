use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::str::contains;
use tempfile::TempDir;

fn write_test_config(root: &Path) -> PathBuf {
    let vault = root.join("vault");
    let templates = vault.join("Templates");
    fs::create_dir_all(&templates).expect("create templates dir");
    fs::write(
        templates.join("AIW_Dev_Log.md"),
        "# {{project_display_name}}\n{{summary}}\n{{transcript_link}}\n{{transcript_excerpt}}\n",
    )
    .expect("write dev log template");
    fs::write(
        templates.join("AIW_ADR.md"),
        "# {{title}}\n## Context\n{{context}}\n## Decision\n{{decision}}\n",
    )
    .expect("write adr template");

    let config_path = root.join("ai-dev-workflow.toml");
    let config = format!(
        "vault_path = \"{}\"\n\
templates_dir = \"Templates\"\n\
dev_log_template = \"AIW_Dev_Log.md\"\n\
adr_template = \"AIW_ADR.md\"\n\
default_transcript_root = \"AI Sessions/raw\"\n\
default_dev_log_root = \"Dev Logs\"\n\
default_adr_root = \"ADR\"\n\
\n\
[tools.claude]\n\
executable = \"printf\"\n\
\n\
[tools.gemini]\n\
executable = \"printf\"\n\
\n\
[tools.codex]\n\
executable = \"printf\"\n\
\n\
[projects.ai-hub]\n\
display_name = \"AI Hub\"\n\
repo_root = \"{}\"\n\
dev_logs_dir = \"Dev Logs/AI Hub\"\n\
adr_dir = \"ADR/AI Hub\"\n\
transcript_dir = \"AI Sessions/raw/AI Hub\"\n\
allowed_note_folders = [\"Projects/AI Hub\"]\n",
        escape_toml_string(vault.to_string_lossy().as_ref()),
        escape_toml_string(root.to_string_lossy().as_ref()),
    );
    fs::write(&config_path, config).expect("write config");
    config_path
}

fn escape_toml_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('\"', "\\\"")
}

fn run_aiw(config: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aiw"));
    cmd.arg("--config").arg(config).args(args);
    cmd.assert()
}

#[test]
fn wrap_status_and_end_flow_creates_transcript_and_dev_log() {
    let temp = TempDir::new().expect("tempdir");
    let config = write_test_config(temp.path());

    run_aiw(
        &config,
        &[
            "session",
            "start",
            "--project",
            "ai-hub",
            "--tool",
            "codex",
            "--topic",
            "integration flow",
            "--wrap",
            "--tool-args",
            "hello from wrapped tool\\n",
        ],
    )
    .success()
    .stdout(contains("Started session:"))
    .stdout(contains("Tool exited with code 0"));

    run_aiw(&config, &["session", "status"])
        .success()
        .stdout(contains("Active session:"))
        .stdout(contains("Capture status: flushed"))
        .stdout(contains("Transcript bytes:"));

    let mut end_cmd = Command::new(env!("CARGO_BIN_EXE_aiw"));
    end_cmd
        .arg("--config")
        .arg(&config)
        .args(["session", "end"])
        .write_stdin("\n\n\n\n\nn\n");
    end_cmd
        .assert()
        .success()
        .stdout(contains("Ended session:"))
        .stdout(contains("Created dev log:"));

    let transcript_root = temp.path().join("vault/AI Sessions/raw/AI Hub");
    let transcripts: Vec<PathBuf> = fs::read_dir(&transcript_root)
        .expect("read transcript date dirs")
        .flat_map(|entry| {
            let path = entry.expect("date dir").path();
            fs::read_dir(path)
                .expect("read transcript files")
                .map(|f| f.expect("transcript entry").path())
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(!transcripts.is_empty(), "expected at least one transcript");
    let transcript_content = fs::read_to_string(&transcripts[0]).expect("read transcript content");
    assert!(transcript_content.contains("hello from wrapped tool"));
    assert!(transcript_content.contains("[aiw] tool exited with code 0"));

    let dev_log_dir = temp.path().join("vault/Dev Logs/AI Hub");
    let dev_logs: Vec<PathBuf> = fs::read_dir(dev_log_dir)
        .expect("read dev logs")
        .map(|entry| entry.expect("dev log entry").path())
        .collect();
    assert!(!dev_logs.is_empty(), "expected dev log file");
}

#[test]
fn doctor_repair_recovers_stale_capturing_session() {
    let temp = TempDir::new().expect("tempdir");
    let config = write_test_config(temp.path());

    run_aiw(
        &config,
        &[
            "session",
            "start",
            "--project",
            "ai-hub",
            "--tool",
            "codex",
            "--topic",
            "repair flow",
        ],
    )
    .success()
    .stdout(contains("Started session:"));

    run_aiw(
        &config,
        &[
            "session",
            "doctor",
            "--project",
            "ai-hub",
            "--tool",
            "codex",
        ],
    )
    .success()
    .stdout(contains("run session doctor with --repair"));

    run_aiw(
        &config,
        &[
            "session",
            "doctor",
            "--project",
            "ai-hub",
            "--tool",
            "codex",
            "--repair",
        ],
    )
    .success()
    .stdout(contains("repair applied: capture state is now recovered"));

    run_aiw(&config, &["session", "status"])
        .success()
        .stdout(contains("Capture status: recovered"));

    let mut end_cmd = Command::new(env!("CARGO_BIN_EXE_aiw"));
    end_cmd
        .arg("--config")
        .arg(&config)
        .args(["session", "end"])
        .write_stdin("\n\n\n\n\nn\n");
    end_cmd.assert().success();
}
