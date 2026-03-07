use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::Result;
use chrono::{Duration, NaiveDate, Utc};

use aiw_config::{resolve_in_vault, Config};

pub struct SearchOptions {
    pub query: String,
    pub project_keys: Vec<String>,
    pub content_type: ContentTypeFilter,
    pub since_days: Option<u32>,
    pub context_lines: usize,
    pub include_transcripts: bool,
    pub extra_folders: Vec<String>,
}

pub enum ContentTypeFilter {
    All,
    DevLogs,
    Adrs,
}

pub struct SearchResult {
    pub vault_relative: String,
    pub project_key: String,
    pub matches: Vec<LineMatch>,
}

pub struct LineMatch {
    pub line_number: usize,
    pub line: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

pub fn search_vault(config: &Config, options: &SearchOptions) -> Result<Vec<SearchResult>> {
    let cutoff = options
        .since_days
        .map(|days| Utc::now().date_naive() - Duration::days(days as i64));

    let project_keys: Vec<&str> = if options.project_keys.is_empty() {
        config.projects.keys().map(String::as_str).collect()
    } else {
        options.project_keys.iter().map(String::as_str).collect()
    };

    let mut results = Vec::new();

    for key in &project_keys {
        let project = match config.projects.get(*key) {
            Some(p) => p,
            None => continue,
        };

        let mut search_dirs: Vec<PathBuf> = Vec::new();

        match options.content_type {
            ContentTypeFilter::DevLogs | ContentTypeFilter::All => {
                search_dirs.push(resolve_in_vault(&config.vault_path, &project.dev_logs_dir)?);
            }
            _ => {}
        }
        match options.content_type {
            ContentTypeFilter::Adrs | ContentTypeFilter::All => {
                search_dirs.push(resolve_in_vault(&config.vault_path, &project.adr_dir)?);
            }
            _ => {}
        }

        if options.include_transcripts {
            search_dirs.push(resolve_in_vault(
                &config.vault_path,
                &project.transcript_dir,
            )?);
        }

        for folder in &project.search_folders {
            search_dirs.push(resolve_in_vault(&config.vault_path, folder)?);
        }

        for extra in &options.extra_folders {
            let p = resolve_in_vault(&config.vault_path, Path::new(extra))?;
            search_dirs.push(p);
        }

        let mut files: Vec<PathBuf> = Vec::new();
        let mut seen_dirs: HashSet<PathBuf> = HashSet::new();
        for dir in &search_dirs {
            if seen_dirs.insert(dir.clone()) {
                collect_md_files(dir, &mut files);
            }
        }

        // Deduplicate files
        let mut seen_files: HashSet<PathBuf> = HashSet::new();
        files.retain(|f| seen_files.insert(f.clone()));

        for file in &files {
            if let Some(cutoff_date) = cutoff {
                let file_date = file_date(file);
                if let Some(d) = file_date {
                    if d < cutoff_date {
                        continue;
                    }
                }
                // If we can't determine date, include file
            }

            let content = match fs::read_to_string(file) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let lines: Vec<String> = content.lines().map(str::to_string).collect();
            let query_lower = options.query.to_lowercase();
            let mut line_matches: Vec<LineMatch> = Vec::new();

            for (i, line) in lines.iter().enumerate() {
                if !line.to_lowercase().contains(&query_lower) {
                    continue;
                }
                let line_number = i + 1;
                let ctx = options.context_lines;

                let before_start = i.saturating_sub(ctx);
                let context_before = lines[before_start..i].to_vec();

                let after_end = (i + 1 + ctx).min(lines.len());
                let context_after = lines[i + 1..after_end].to_vec();

                line_matches.push(LineMatch {
                    line_number,
                    line: line.clone(),
                    context_before,
                    context_after,
                });
            }

            if !line_matches.is_empty() {
                let vault_relative = file
                    .strip_prefix(&config.vault_path)
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|_| file.to_string_lossy().into_owned());

                results.push(SearchResult {
                    vault_relative,
                    project_key: key.to_string(),
                    matches: line_matches,
                });
            }
        }
    }

    Ok(results)
}

fn collect_md_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

fn file_date(path: &Path) -> Option<NaiveDate> {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if let Some(date) = extract_date_from_filename(name) {
            return Some(date);
        }
    }
    // Fall back to filesystem mtime
    let meta = fs::metadata(path).ok()?;
    let modified: SystemTime = meta.modified().ok()?;
    let secs = modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    chrono::DateTime::from_timestamp(secs, 0).map(|dt| dt.date_naive())
}

fn extract_date_from_filename(name: &str) -> Option<NaiveDate> {
    // Find first occurrence of YYYY-MM-DD pattern
    for i in 0..name.len().saturating_sub(9) {
        let slice = &name[i..i + 10];
        if let Ok(d) = NaiveDate::parse_from_str(slice, "%Y-%m-%d") {
            return Some(d);
        }
    }
    None
}
