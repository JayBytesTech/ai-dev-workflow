use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};

use aiw_config::{resolve_in_vault, Config, ProjectConfig};

pub mod search;
pub use search::{search_vault, ContentTypeFilter, LineMatch, SearchOptions, SearchResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteCommand {
    Summarize,
    Critique,
    Research,
    ExtractTasks,
}

#[derive(Debug, Clone)]
pub struct NoteCommandMatch {
    pub command: NoteCommand,
    pub line: usize,
    pub raw: String,
}

pub fn resolve_note_path(
    config: &Config,
    project: &ProjectConfig,
    note_path: &Path,
) -> Result<PathBuf> {
    let resolved = resolve_in_vault(&config.vault_path, note_path)?;
    if !is_allowed_note_path(config, project, &resolved)? {
        return Err(anyhow!(
            "note path is not within allowed folders: {}",
            resolved.display()
        ));
    }
    Ok(resolved)
}

pub fn scan_note_for_commands(path: &Path) -> Result<Vec<NoteCommandMatch>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read note {}", path.display()))?;
    Ok(scan_content_for_commands(&content))
}

pub fn scan_content_for_commands(content: &str) -> Vec<NoteCommandMatch> {
    let mut matches = Vec::new();
    for (index, line) in content.lines().enumerate() {
        if let Some(cmd) = parse_command(line) {
            matches.push(NoteCommandMatch {
                command: cmd,
                line: index + 1,
                raw: line.trim().to_string(),
            });
        }
    }
    matches
}

fn parse_command(line: &str) -> Option<NoteCommand> {
    let trimmed = line.trim();
    if !trimmed.starts_with("/ai ") {
        return None;
    }
    match trimmed {
        "/ai summarize" => Some(NoteCommand::Summarize),
        "/ai critique" => Some(NoteCommand::Critique),
        "/ai research" => Some(NoteCommand::Research),
        "/ai extract-tasks" => Some(NoteCommand::ExtractTasks),
        _ => None,
    }
}

fn is_allowed_note_path(config: &Config, project: &ProjectConfig, note: &Path) -> Result<bool> {
    let note = note.canonicalize().unwrap_or_else(|_| note.to_path_buf());
    for folder in &project.allowed_note_folders {
        let resolved = resolve_in_vault(&config.vault_path, folder)?;
        let resolved = resolved
            .canonicalize()
            .unwrap_or_else(|_| resolved.to_path_buf());
        if note.starts_with(&resolved) {
            return Ok(true);
        }
    }
    Ok(false)
}
