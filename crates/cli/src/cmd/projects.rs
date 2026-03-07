use std::path::Path;

use anyhow::Result;

use crate::ProjectsCommands;

use super::resolve_config_path;

pub(crate) fn handle_projects(
    cmd: ProjectsCommands,
    config_path: Option<&Path>,
    profile: Option<&str>,
) -> Result<()> {
    match cmd {
        ProjectsCommands::List => {
            let config_path = resolve_config_path(config_path)?;
            let config = aiw_config::Config::load_with_profile(&config_path, profile)?;
            if config.projects.is_empty() {
                println!("No projects configured.");
                return Ok(());
            }
            println!("Configured projects:");
            for (key, project) in config.projects {
                println!("- {} ({})", key, project.display_name);
            }
            Ok(())
        }
    }
}
