use std::path::Path;

use anyhow::Result;

use crate::SearchArgs;

use super::resolve_config_path;

pub(crate) fn handle_search(
    args: SearchArgs,
    config_path: Option<&Path>,
    profile: Option<&str>,
) -> Result<()> {
    let config_path = resolve_config_path(config_path)?;
    let config = aiw_config::Config::load_with_profile(&config_path, profile)?;
    let report = config.validate();
    if !report.is_ok() {
        println!("{report}");
        return Err(anyhow::anyhow!("config validation failed"));
    }

    let project_keys = match &args.project {
        Some(k) => vec![k.clone()],
        None => config.projects.keys().cloned().collect(),
    };

    let options = aiw_obsidian::SearchOptions {
        query: args.query,
        project_keys,
        content_type: args.content_type.into(),
        since_days: args.since,
        context_lines: args.context,
        include_transcripts: args.include_transcripts,
        extra_folders: args.folders,
    };

    let results = aiw_obsidian::search_vault(&config, &options)?;

    if results.is_empty() {
        println!("No matches found.");
        return Ok(());
    }

    for result in &results {
        println!("\n── {} ──", result.vault_relative);
        for m in &result.matches {
            for (i, line) in m.context_before.iter().enumerate() {
                let n = m.line_number - m.context_before.len() + i;
                println!("  {:>4}  {}", n, line);
            }
            println!("  {:>4}: {}", m.line_number, m.line);
            for (i, line) in m.context_after.iter().enumerate() {
                println!("  {:>4}  {}", m.line_number + 1 + i, line);
            }
        }
    }

    println!("\n{} file(s) matched.", results.len());
    Ok(())
}
