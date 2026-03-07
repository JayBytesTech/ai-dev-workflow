use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::str::contains;
use serde_json::Value;
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

fn write_test_config_with_claude(root: &Path, claude_exe: &Path) -> PathBuf {
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
executable = \"{}\"\n\
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
        escape_toml_string(claude_exe.to_string_lossy().as_ref()),
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

fn write_profile_config(root: &Path) -> PathBuf {
    let vault = root.join("vault");
    let vault_ci = root.join("vault-ci");
    let templates = vault.join("Templates");
    let templates_ci = vault_ci.join("Templates");
    fs::create_dir_all(&templates).expect("create templates dir");
    fs::create_dir_all(&templates_ci).expect("create ci templates dir");
    fs::write(templates.join("AIW_Dev_Log.md"), "base").expect("write base template");
    fs::write(templates.join("AIW_ADR.md"), "base").expect("write base template");
    fs::write(templates_ci.join("AIW_Dev_Log.md"), "ci").expect("write ci template");
    fs::write(templates_ci.join("AIW_ADR.md"), "ci").expect("write ci template");

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
allowed_note_folders = [\"Projects/AI Hub\"]\n\
\n\
[profiles.ci]\n\
vault_path = \"{}\"\n\
\n\
[profiles.ci.projects.ai-hub]\n\
transcript_dir = \"AI Sessions/raw/AI Hub/CI\"\n",
        escape_toml_string(vault.to_string_lossy().as_ref()),
        escape_toml_string(root.to_string_lossy().as_ref()),
        escape_toml_string(vault_ci.to_string_lossy().as_ref()),
    );
    fs::write(&config_path, config).expect("write config");
    config_path
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

#[test]
fn session_end_non_interactive_json_output_and_adr_flags() {
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
            "json flow",
        ],
    )
    .success();

    let output = Command::new(env!("CARGO_BIN_EXE_aiw"))
        .arg("--config")
        .arg(&config)
        .args([
            "session",
            "end",
            "--non-interactive",
            "--output",
            "json",
            "--goal",
            "Ship non-interactive mode",
            "--summary",
            "Implemented non-interactive session end",
            "--decision",
            "Use explicit CLI flags for automation",
            "--rationale",
            "Allow CI-safe execution with no prompts",
            "--follow-up-task",
            "add docs",
            "--adr-title",
            "Automate session end in CI",
            "--adr-context",
            "Need reproducible non-interactive workflow",
            "--adr-options",
            "interactive prompts vs explicit flags",
            "--adr-decision",
            "Use explicit flags and JSON output",
            "--adr-consequences",
            "Slightly longer command lines but scriptable",
        ])
        .output()
        .expect("run session end");
    assert!(output.status.success(), "session end should succeed");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let payload: Value = serde_json::from_str(&stdout).expect("json output");
    assert_eq!(
        payload["project"].as_str(),
        Some("AI Hub"),
        "project should be serialized"
    );
    assert_eq!(
        payload["capture_status"].as_str(),
        Some("flushed"),
        "capture should be terminal"
    );
    assert!(
        payload["dev_log_path"]
            .as_str()
            .unwrap_or_default()
            .contains("Dev Logs/AI Hub"),
        "dev log path should be returned"
    );
    assert!(
        payload["adr_path"]
            .as_str()
            .unwrap_or_default()
            .contains("ADR/AI Hub"),
        "adr path should be returned"
    );
}

#[test]
fn session_end_auto_adr_non_interactive_uses_auto_tool() {
    let temp = TempDir::new().expect("tempdir");
    let bin_dir = temp.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("create bin dir");
    let tool_path = bin_dir.join("auto-adr-tool.sh");
    fs::write(
        &tool_path,
        "#!/bin/sh\ncat > /dev/null\nprintf '{\"title\":\"Auto ADR\",\"context\":\"ctx\",\"options\":\"- A\",\"decision\":\"dec\",\"consequences\":\"cons\"}'\n",
    )
    .expect("write tool script");
    let mut perms = fs::metadata(&tool_path).expect("metadata").permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
        fs::set_permissions(&tool_path, perms).expect("set perms");
    }

    let config = write_test_config_with_claude(temp.path(), &tool_path);

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
            "auto adr",
        ],
    )
    .success();

    let state_path = temp.path().join(".aiw").join("session.json");
    let state_raw = fs::read_to_string(&state_path).expect("read session state");
    let state_json: Value = serde_json::from_str(&state_raw).expect("parse session state");
    let transcript_path = state_json["transcript_path"]
        .as_str()
        .expect("transcript path");
    let transcript_path = PathBuf::from(transcript_path);
    if let Some(parent) = transcript_path.parent() {
        fs::create_dir_all(parent).expect("create transcript dir");
    }
    fs::write(&transcript_path, "session content").expect("write transcript");

    let output = Command::new(env!("CARGO_BIN_EXE_aiw"))
        .arg("--config")
        .arg(&config)
        .args([
            "session",
            "end",
            "--non-interactive",
            "--output",
            "json",
            "--auto-adr",
            "--auto-tool",
            "claude",
        ])
        .output()
        .expect("run session end");
    assert!(output.status.success(), "session end should succeed");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    let payload: Value = serde_json::from_str(&stdout).expect("json output");
    assert!(
        payload["adr_path"]
            .as_str()
            .unwrap_or_default()
            .contains("ADR/AI Hub"),
        "adr path should be returned"
    );
}

#[test]
fn session_end_auto_adr_interactive_accepts_defaults() {
    let temp = TempDir::new().expect("tempdir");
    let bin_dir = temp.path().join("bin");
    fs::create_dir_all(&bin_dir).expect("create bin dir");
    let tool_path = bin_dir.join("auto-adr-tool.sh");
    fs::write(
        &tool_path,
        "#!/bin/sh\ncat > /dev/null\nprintf '{\"title\":\"Auto ADR\",\"context\":\"ctx\",\"options\":\"- A\",\"decision\":\"dec\",\"consequences\":\"cons\"}'\n",
    )
    .expect("write tool script");
    let mut perms = fs::metadata(&tool_path).expect("metadata").permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o755);
        fs::set_permissions(&tool_path, perms).expect("set perms");
    }

    let config = write_test_config_with_claude(temp.path(), &tool_path);

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
            "auto adr interactive",
        ],
    )
    .success();

    let state_path = temp.path().join(".aiw").join("session.json");
    let state_raw = fs::read_to_string(&state_path).expect("read session state");
    let state_json: Value = serde_json::from_str(&state_raw).expect("parse session state");
    let transcript_path = state_json["transcript_path"]
        .as_str()
        .expect("transcript path");
    let transcript_path = PathBuf::from(transcript_path);
    if let Some(parent) = transcript_path.parent() {
        fs::create_dir_all(parent).expect("create transcript dir");
    }
    fs::write(&transcript_path, "session content").expect("write transcript");

    let mut end_cmd = Command::new(env!("CARGO_BIN_EXE_aiw"));
    end_cmd
        .arg("--config")
        .arg(&config)
        .args(["session", "end", "--auto-adr", "--auto-tool", "claude"])
        .write_stdin("\n\n\n\n\n\n\n\n\n\n");
    let output = end_cmd.output().expect("run session end");
    assert!(output.status.success(), "session end should succeed");
    let adr_dir = temp.path().join("vault").join("ADR").join("AI Hub");
    let mut entries = fs::read_dir(&adr_dir)
        .expect("read adr dir")
        .filter_map(|entry| entry.ok())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());
    let adr_path = entries.last().expect("adr file").path();
    let adr_body = fs::read_to_string(&adr_path).expect("read adr");
    assert!(
        adr_body.contains("Auto ADR"),
        "adr should use auto-generated defaults"
    );
}

#[test]
fn config_show_resolved_applies_profile() {
    let temp = TempDir::new().expect("tempdir");
    let config = write_profile_config(temp.path());

    let output = Command::new(env!("CARGO_BIN_EXE_aiw"))
        .arg("--config")
        .arg(&config)
        .arg("--profile")
        .arg("ci")
        .args(["config", "show", "--resolved"])
        .output()
        .expect("run config show");
    assert!(output.status.success(), "config show should succeed");
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("vault-ci"));
    assert!(stdout.contains("AI Sessions/raw/AI Hub/CI"));
}
