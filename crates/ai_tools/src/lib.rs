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
        Ok(Self { kind: tool, executable })
    }
}

#[derive(Debug)]
pub struct ToolOutput {
    pub stdout: String,
    pub stderr: String,
    pub status: i32,
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
