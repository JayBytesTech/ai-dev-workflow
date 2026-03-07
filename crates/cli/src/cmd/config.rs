use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::ConfigCommands;

use super::{resolve_config_path, DEFAULT_CONFIG_FILE};

pub(crate) fn handle_config(
    cmd: ConfigCommands,
    config_path: Option<&Path>,
    profile: Option<&str>,
) -> Result<()> {
    match cmd {
        ConfigCommands::Init(args) => {
            let target = args
                .output
                .or_else(|| config_path.map(PathBuf::from))
                .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_FILE));
            write_sample_config(&target)?;
            println!("Created config: {}", target.display());
            Ok(())
        }
        ConfigCommands::Validate(_) => {
            let config_path = resolve_config_path(config_path)?;
            let config = aiw_config::Config::load_with_profile(&config_path, profile)?;
            let report = config.validate();
            println!("{report}");
            if report.is_ok() {
                Ok(())
            } else {
                Err(anyhow::anyhow!("config validation failed"))
            }
        }
        ConfigCommands::Show(args) => {
            let config_path = resolve_config_path(config_path)?;
            if args.resolved {
                let config = aiw_config::Config::load_with_profile(&config_path, profile)?;
                let rendered = toml::to_string_pretty(&config)
                    .context("Failed to serialize resolved config")?;
                print!("{rendered}");
            } else {
                let raw = fs::read_to_string(&config_path)
                    .with_context(|| format!("Failed to read {}", config_path.display()))?;
                print!("{raw}");
            }
            Ok(())
        }
    }
}

fn write_sample_config(path: &Path) -> Result<()> {
    if path.exists() {
        return Err(anyhow::anyhow!(
            "config already exists at {}",
            path.display()
        ));
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }
    }
    let sample = include_str!("../../../../config/ai-dev-workflow.example.toml");
    fs::write(path, sample).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}
