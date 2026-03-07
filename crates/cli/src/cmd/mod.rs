pub(crate) mod adr;
pub(crate) mod config;
pub(crate) mod note;
pub(crate) mod projects;
pub(crate) mod search;
pub(crate) mod session;

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub(crate) const DEFAULT_CONFIG_FILE: &str = "ai-dev-workflow.toml";

pub(crate) fn resolve_config_path(config_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = config_path {
        return Ok(path.to_path_buf());
    }
    if let Ok(path) = std::env::var("AIW_CONFIG") {
        return Ok(PathBuf::from(path));
    }
    Ok(PathBuf::from(DEFAULT_CONFIG_FILE))
}

pub(crate) fn session_state_dir(config_path: &Path, profile: Option<&str>) -> PathBuf {
    let base = config_path.parent().unwrap_or_else(|| Path::new("."));
    let mut dir = base.join(".aiw");
    if let Some(name) = profile {
        dir = dir.join(name);
    }
    dir
}

pub(crate) fn prompt_line(label: &str) -> Result<String> {
    print!("{label}: ");
    io::stdout().flush().context("Failed to flush stdout")?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("Failed to read input")?;
    Ok(input.trim().to_string())
}

pub(crate) fn prompt_line_with_default(label: &str, default: &str) -> Result<String> {
    print!("{label} [{default}]: ");
    io::stdout().flush().context("Failed to flush stdout")?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("Failed to read input")?;
    let trimmed = input.trim();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

pub(crate) fn prompt_yes_no(label: &str) -> Result<bool> {
    let response = prompt_line(label)?;
    let response = response.trim().to_ascii_lowercase();
    Ok(matches!(response.as_str(), "y" | "yes"))
}
