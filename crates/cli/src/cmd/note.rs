use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::NoteCommands;

use super::resolve_config_path;

pub(crate) fn handle_note(
    cmd: NoteCommands,
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

    match cmd {
        NoteCommands::Scan(args) => {
            let project = config
                .projects
                .get(&args.project)
                .ok_or_else(|| anyhow::anyhow!("project not found: {}", args.project))?;
            let resolved = aiw_obsidian::resolve_note_path(&config, project, &args.path)?;
            let matches = aiw_obsidian::scan_note_for_commands(&resolved)?;
            if matches.is_empty() {
                println!("No commands found in {}", resolved.display());
                return Ok(());
            }
            println!("Found commands in {}:", resolved.display());
            for item in matches {
                println!("- line {}: {}", item.line, item.raw);
            }
            Ok(())
        }
        NoteCommands::Process(args) => {
            let project = config
                .projects
                .get(&args.project)
                .ok_or_else(|| anyhow::anyhow!("project not found: {}", args.project))?;
            let resolved = aiw_obsidian::resolve_note_path(&config, project, &args.path)?;
            let content = fs::read_to_string(&resolved)
                .with_context(|| format!("Failed to read note {}", resolved.display()))?;
            let commands = aiw_obsidian::scan_content_for_commands(&content);
            if commands.is_empty() {
                println!("No commands found in {}", resolved.display());
                return Ok(());
            }

            let tool_kind = aiw_ai_tools::ToolKind::parse(&args.tool)?;
            let adapter = aiw_ai_tools::ToolAdapter::from_config(&config, tool_kind)?;

            let mut appended_blocks = String::new();
            let mut processed = 0usize;
            for cmd in &commands {
                let marker = build_note_marker(cmd, &content);
                if content.contains(&marker) {
                    continue;
                }
                let prompt = build_note_prompt(&cmd.raw, &content);
                let output = aiw_ai_tools::run_prompt(&adapter, &prompt)?;
                let block = format_note_result_block(cmd, &marker, &output.stdout, &output.stderr);
                appended_blocks.push_str(&block);
                processed += 1;
            }

            if processed == 0 {
                println!("All commands already processed in {}", resolved.display());
                return Ok(());
            }

            let mut updated = content;
            if !updated.ends_with('\n') {
                updated.push('\n');
            }
            updated.push_str(&appended_blocks);
            fs::write(&resolved, updated)
                .with_context(|| format!("Failed to write note {}", resolved.display()))?;
            println!(
                "Processed {} command(s) in {}",
                processed,
                resolved.display()
            );
            Ok(())
        }
    }
}

fn build_note_prompt(command: &str, content: &str) -> String {
    format!(
        "You are assisting with a markdown note.\nCommand: {command}\n\nNote content:\n{content}\n"
    )
}

fn format_note_result_block(
    cmd: &aiw_obsidian::NoteCommandMatch,
    marker: &str,
    stdout: &str,
    stderr: &str,
) -> String {
    let mut block = String::new();
    block.push_str("\n---\n");
    block.push_str("## AIW Results\n\n");
    block.push_str(marker);
    block.push('\n');
    block.push_str(&format!(
        "**AI Result ({})**\n\n",
        note_command_label(&cmd.command)
    ));
    match cmd.command {
        aiw_obsidian::NoteCommand::ExtractTasks => {
            let body = format_tasks(stdout);
            block.push_str(body.trim());
            block.push('\n');
        }
        _ => {
            let body = stdout.trim();
            if !body.is_empty() {
                block.push_str("```text\n");
                block.push_str(body);
                block.push_str("\n```\n");
            }
        }
    }
    if !stderr.trim().is_empty() {
        block.push_str("\n**Tool Stderr**\n\n```text\n");
        block.push_str(stderr.trim());
        block.push_str("\n```\n");
    }
    block
}

fn format_tasks(stdout: &str) -> String {
    let mut tasks = Vec::new();
    for line in stdout.lines() {
        let cleaned = line
            .trim()
            .trim_start_matches("- [ ]")
            .trim_start_matches("- [x]")
            .trim_start_matches("- [X]")
            .trim_start_matches("-")
            .trim_start_matches("*")
            .trim_start_matches("•")
            .trim_start_matches(char::is_numeric)
            .trim_start_matches(['.', ')'])
            .trim();
        if cleaned.is_empty() {
            continue;
        }
        tasks.push(format!("- [ ] {cleaned}"));
    }
    if tasks.is_empty() {
        tasks.push("- [ ] (no tasks extracted)".to_string());
    }
    tasks.join("\n")
}

fn build_note_marker(cmd: &aiw_obsidian::NoteCommandMatch, note_content: &str) -> String {
    let mut input = String::new();
    input.push_str(cmd.raw.as_str());
    input.push('|');
    input.push_str(cmd.line.to_string().as_str());
    input.push('|');
    input.push_str(&stable_hash(note_content).to_string());
    let hash = stable_hash(&input);
    format!(
        "<!-- AIW_RESULT: {} {} -->",
        note_command_label(&cmd.command),
        hash
    )
}

fn stable_hash(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn note_command_label(command: &aiw_obsidian::NoteCommand) -> &'static str {
    match command {
        aiw_obsidian::NoteCommand::Summarize => "summarize",
        aiw_obsidian::NoteCommand::Critique => "critique",
        aiw_obsidian::NoteCommand::Research => "research",
        aiw_obsidian::NoteCommand::ExtractTasks => "extract-tasks",
    }
}
