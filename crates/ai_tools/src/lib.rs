use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::{anyhow, Context, Result};

use aiw_config::Config;

#[derive(Debug, Clone, Copy)]
pub enum ToolKind {
    Claude,
    Gemini,
    Codex,
}

impl ToolKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ToolKind::Claude => "claude",
            ToolKind::Gemini => "gemini",
            ToolKind::Codex => "codex",
        }
    }

    pub fn parse(input: &str) -> Result<Self> {
        match input.to_ascii_lowercase().as_str() {
            "claude" => Ok(ToolKind::Claude),
            "gemini" => Ok(ToolKind::Gemini),
            "codex" => Ok(ToolKind::Codex),
            _ => Err(anyhow!("unsupported tool: {input}")),
        }
    }
}

#[derive(Debug)]
pub struct ToolAdapter {
    pub kind: ToolKind,
    pub executable: String,
}

impl ToolAdapter {
    pub fn from_config(config: &Config, tool: ToolKind) -> Result<Self> {
        let executable = match tool {
            ToolKind::Claude => config.tools.claude.executable.clone(),
            ToolKind::Gemini => config.tools.gemini.executable.clone(),
            ToolKind::Codex => config.tools.codex.executable.clone(),
        };
        if executable.trim().is_empty() {
            return Err(anyhow!("tool executable is empty: {}", tool.as_str()));
        }
        Ok(Self {
            kind: tool,
            executable,
        })
    }
}

#[derive(Debug)]
pub struct ToolOutput {
    pub stdout: String,
    pub stderr: String,
    pub status: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiw_config::{Config, ProjectConfig, ToolConfig, ToolsConfig};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_config(claude_exe: &str, gemini_exe: &str, codex_exe: &str) -> Config {
        Config {
            vault_path: PathBuf::from("/tmp/vault"),
            templates_dir: PathBuf::from("templates"),
            dev_log_template: PathBuf::from("dev_log.md"),
            adr_template: PathBuf::from("adr.md"),
            default_transcript_root: PathBuf::from("transcripts"),
            default_dev_log_root: PathBuf::from("dev_logs"),
            default_adr_root: PathBuf::from("adrs"),
            tools: ToolsConfig {
                claude: ToolConfig {
                    executable: claude_exe.to_string(),
                },
                gemini: ToolConfig {
                    executable: gemini_exe.to_string(),
                },
                codex: ToolConfig {
                    executable: codex_exe.to_string(),
                },
            },
            projects: HashMap::new(),
        }
    }

    // --- ToolKind::parse ---

    #[test]
    fn parse_known_tools() {
        assert!(matches!(
            ToolKind::parse("claude").unwrap(),
            ToolKind::Claude
        ));
        assert!(matches!(
            ToolKind::parse("gemini").unwrap(),
            ToolKind::Gemini
        ));
        assert!(matches!(ToolKind::parse("codex").unwrap(), ToolKind::Codex));
    }

    #[test]
    fn parse_is_case_insensitive() {
        assert!(matches!(
            ToolKind::parse("Claude").unwrap(),
            ToolKind::Claude
        ));
        assert!(matches!(
            ToolKind::parse("GEMINI").unwrap(),
            ToolKind::Gemini
        ));
        assert!(matches!(ToolKind::parse("CODEX").unwrap(), ToolKind::Codex));
    }

    #[test]
    fn parse_unknown_tool_returns_error() {
        let err = ToolKind::parse("gpt4").unwrap_err().to_string();
        assert!(err.contains("unsupported tool: gpt4"));
    }

    // --- ToolKind::as_str ---

    #[test]
    fn as_str_round_trips() {
        assert_eq!(ToolKind::Claude.as_str(), "claude");
        assert_eq!(ToolKind::Gemini.as_str(), "gemini");
        assert_eq!(ToolKind::Codex.as_str(), "codex");
    }

    // --- ToolAdapter::from_config ---

    #[test]
    fn from_config_returns_adapter_with_correct_executable() {
        let config = make_config("claude-bin", "gemini-bin", "codex-bin");
        let adapter = ToolAdapter::from_config(&config, ToolKind::Gemini).unwrap();
        assert_eq!(adapter.executable, "gemini-bin");
    }

    #[test]
    fn from_config_empty_executable_returns_error() {
        let config = make_config("", "gemini-bin", "codex-bin");
        let err = ToolAdapter::from_config(&config, ToolKind::Claude)
            .unwrap_err()
            .to_string();
        assert!(err.contains("tool executable is empty: claude"));
    }

    #[test]
    fn from_config_whitespace_only_executable_returns_error() {
        let config = make_config("claude-bin", "  ", "codex-bin");
        let err = ToolAdapter::from_config(&config, ToolKind::Gemini)
            .unwrap_err()
            .to_string();
        assert!(err.contains("tool executable is empty: gemini"));
    }
}

pub fn run_prompt(adapter: &ToolAdapter, prompt: &str) -> Result<ToolOutput> {
    let mut child = Command::new(&adapter.executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to spawn {}", adapter.executable))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .with_context(|| "Failed to write prompt to stdin")?;
    }

    let output = child
        .wait_with_output()
        .with_context(|| "Failed to read tool output")?;

    Ok(ToolOutput {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        status: output.status.code().unwrap_or(-1),
    })
}
