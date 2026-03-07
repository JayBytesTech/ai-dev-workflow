use std::path::Path;

use anyhow::Result;

use crate::AdrCommands;

use super::{prompt_line, prompt_line_with_default, resolve_config_path};

pub(crate) fn handle_adr(
    cmd: AdrCommands,
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
        AdrCommands::Create(args) => {
            let project = config
                .projects
                .get(&args.project)
                .ok_or_else(|| anyhow::anyhow!("project not found: {}", args.project))?;
            let adr_input = prompt_adr_input(Some(args.title))?;
            let adr_path = aiw_adr::create_adr(&config, project, adr_input)?;
            println!("Created ADR: {}", adr_path.display());
            Ok(())
        }
    }
}

pub(crate) fn prompt_adr_input(title: Option<String>) -> Result<aiw_adr::AdrInput> {
    let title = match title {
        Some(value) => value,
        None => prompt_line("ADR Title")?,
    };
    let context = prompt_line("Context")?;
    let options = prompt_line("Options considered")?;
    let decision = prompt_line("Decision")?;
    let consequences = prompt_line("Consequences")?;

    Ok(aiw_adr::AdrInput {
        title,
        context,
        options,
        decision,
        consequences,
    })
}

pub(crate) fn prompt_adr_input_with_defaults(
    defaults: &aiw_adr::AdrInput,
) -> Result<aiw_adr::AdrInput> {
    let title = prompt_line_with_default("ADR Title", &defaults.title)?;
    let context = prompt_line_with_default("Context", &defaults.context)?;
    let options = prompt_line_with_default("Options considered", &defaults.options)?;
    let decision = prompt_line_with_default("Decision", &defaults.decision)?;
    let consequences = prompt_line_with_default("Consequences", &defaults.consequences)?;

    Ok(aiw_adr::AdrInput {
        title,
        context,
        options,
        decision,
        consequences,
    })
}
