use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Local;

use aiw_config::{resolve_in_vault, Config, ProjectConfig};
use aiw_templates::{render_template, TemplateStore};

#[derive(Debug)]
pub struct AdrInput {
    pub title: String,
    pub context: String,
    pub options: String,
    pub decision: String,
    pub consequences: String,
}

pub fn create_adr(config: &Config, project: &ProjectConfig, input: AdrInput) -> Result<PathBuf> {
    let templates_root = resolve_in_vault(&config.vault_path, &config.templates_dir)?;
    let store = TemplateStore::new(templates_root);
    let template = store.load(&config.adr_template)?;

    let adr_root = resolve_in_vault(&config.vault_path, &project.adr_dir)?;
    let adr_number = next_adr_number(&adr_root)?;
    let date = Local::now().format("%Y-%m-%d").to_string();

    let mut values = HashMap::new();
    values.insert("adr_number", format!("{adr_number:04}"));
    values.insert("title", input.title.clone());
    values.insert("date", date);
    values.insert("project_display_name", project.display_name.clone());
    values.insert("context", input.context);
    values.insert("options", input.options);
    values.insert("decision", input.decision);
    values.insert("consequences", input.consequences);

    let rendered = render_template(&template, &values);

    let filename = format!("ADR-{:04}-{}.md", adr_number, slugify(&input.title));
    let path = adr_root.join(filename);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create ADR directory {}", parent.display()))?;
    }
    fs::write(&path, rendered).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(path)
}

fn next_adr_number(root: &Path) -> Result<u32> {
    if !root.exists() {
        return Ok(1);
    }
    let mut max_seen = 0u32;
    for entry in fs::read_dir(root).with_context(|| "Failed to read ADR directory")? {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if let Some(number) = parse_adr_number(&file_name) {
            if number > max_seen {
                max_seen = number;
            }
        }
    }
    Ok(max_seen + 1)
}

fn parse_adr_number(name: &str) -> Option<u32> {
    if !name.starts_with("ADR-") {
        return None;
    }
    let rest = &name[4..];
    let mut chars = rest.chars();
    let mut digits = String::new();
    for _ in 0..4 {
        if let Some(c) = chars.next() {
            if c.is_ascii_digit() {
                digits.push(c);
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    digits.parse().ok()
}

fn slugify(title: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in title.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "adr".to_string()
    } else {
        trimmed
    }
}
