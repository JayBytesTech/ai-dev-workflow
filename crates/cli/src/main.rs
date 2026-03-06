use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};

const DEFAULT_CONFIG_FILE: &str = "ai-dev-workflow.toml";

#[derive(Parser)]
#[command(name = "aiw", version, about = "AI development workflow helper")]
struct Cli {
    #[arg(long, value_name = "path")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Config(ConfigCommands),
    Session(SessionCommands),
    Note(NoteCommands),
    Adr(AdrCommands),
    Projects(ProjectsCommands),
}

#[derive(Subcommand)]
enum ConfigCommands {
    Init(ConfigInitArgs),
    Validate(ConfigValidateArgs),
}

#[derive(Args)]
struct ConfigInitArgs {
    #[arg(long, value_name = "path")]
    output: Option<PathBuf>,
}

#[derive(Args)]
struct ConfigValidateArgs {}

#[derive(Subcommand)]
enum SessionCommands {
    Start(SessionStartArgs),
    End,
    Status,
}

#[derive(Args)]
struct SessionStartArgs {
    #[arg(long, value_name = "name")]
    project: String,
    #[arg(long, value_name = "tool")]
    tool: String,
    #[arg(long, value_name = "text")]
    topic: Option<String>,
    #[arg(long, value_name = "path")]
    cwd: Option<PathBuf>,
    #[arg(long, help = "Run the tool and capture stdout/stderr to the transcript file")]
    wrap: bool,
}

#[derive(Subcommand)]
enum NoteCommands {
    Scan(NoteScanArgs),
    Process(NoteProcessArgs),
}

#[derive(Args)]
struct NoteScanArgs {
    #[arg(long, value_name = "name")]
    project: String,
    #[arg(long, value_name = "path")]
    path: PathBuf,
}

#[derive(Args)]
struct NoteProcessArgs {
    #[arg(long, value_name = "name")]
    project: String,
    #[arg(long, value_name = "tool")]
    tool: String,
    #[arg(long, value_name = "path")]
    path: PathBuf,
}

#[derive(Subcommand)]
enum AdrCommands {
    Create(AdrCreateArgs),
}

#[derive(Args)]
struct AdrCreateArgs {
    #[arg(long, value_name = "name")]
    project: String,
    #[arg(long, value_name = "text")]
    title: String,
}

#[derive(Subcommand)]
enum ProjectsCommands {
    List,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Config(cmd) => handle_config(cmd, cli.config.as_deref()),
        Commands::Projects(cmd) => handle_projects(cmd, cli.config.as_deref()),
        Commands::Session(cmd) => handle_session(cmd, cli.config.as_deref()),
        Commands::Note(cmd) => handle_note(cmd, cli.config.as_deref()),
        Commands::Adr(cmd) => handle_adr(cmd, cli.config.as_deref()),
    }
}

fn handle_config(cmd: ConfigCommands, config_path: Option<&Path>) -> Result<()> {
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
            let config = aiw_config::Config::load(&config_path)?;
            let report = config.validate();
            println!("{report}");
            if report.is_ok() {
                Ok(())
            } else {
                Err(anyhow::anyhow!("config validation failed"))
            }
        }
    }
}

fn handle_projects(cmd: ProjectsCommands, config_path: Option<&Path>) -> Result<()> {
    match cmd {
        ProjectsCommands::List => {
            let config_path = resolve_config_path(config_path)?;
            let config = aiw_config::Config::load(&config_path)?;
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

fn handle_session(cmd: SessionCommands, config_path: Option<&Path>) -> Result<()> {
    let config_path = resolve_config_path(config_path)?;
    let state_dir = session_state_dir(&config_path)?;
    let store = aiw_session::SessionStore::new(state_dir)?;

    match cmd {
        SessionCommands::Start(args) => {
            let config = aiw_config::Config::load(&config_path)?;
            let report = config.validate();
            if !report.is_ok() {
                println!("{report}");
                return Err(anyhow::anyhow!("config validation failed"));
            }
            let cwd = match args.cwd {
                Some(path) => path,
                None => std::env::current_dir().context("Failed to determine current directory")?,
            };
            let state = aiw_session::start_session(
                &config,
                &args.project,
                &args.tool,
                args.topic,
                cwd,
                &store,
            )?;
            println!("Started session: {}", state.id);
            println!("Project: {}", state.project_display_name);
            println!("Tool: {}", state.tool);
            println!("Transcript target: {}", state.transcript_path.display());

            if args.wrap {
                let tool_kind = aiw_ai_tools::ToolKind::parse(&args.tool)?;
                let adapter = aiw_ai_tools::ToolAdapter::from_config(&config, tool_kind)?;
                println!("Launching tool: {}", adapter.executable);
                let code = aiw_session::run_tool_with_transcript(
                    &adapter.executable,
                    &state.transcript_path,
                )?;
                println!("Tool exited with code {code}");
            }
            Ok(())
        }
        SessionCommands::End => {
            let config = aiw_config::Config::load(&config_path)?;
            let report = config.validate();
            if !report.is_ok() {
                println!("{report}");
                return Err(anyhow::anyhow!("config validation failed"));
            }
            let state = aiw_session::end_session(&store)?;
            let project = config
                .projects
                .get(&state.project_key)
                .ok_or_else(|| anyhow::anyhow!("project not found: {}", state.project_key))?;

            let input = prompt_dev_log_input()?;
            let git_info = aiw_session::collect_git_info(project);
            let dev_log_path =
                aiw_session::write_dev_log(&config, project, &state, input, git_info)?;

            println!("Ended session: {}", state.id);
            println!("Project: {}", state.project_display_name);
            println!("Tool: {}", state.tool);
            println!("Transcript: {}", state.transcript_path.display());
            println!("Created dev log: {}", dev_log_path.display());

            if prompt_yes_no("Create ADR? (y/N)")? {
                let adr_input = prompt_adr_input(None)?;
                let adr_path = aiw_adr::create_adr(&config, project, adr_input)?;
                println!("Created ADR: {}", adr_path.display());
            }
            Ok(())
        }
        SessionCommands::Status => {
            match aiw_session::session_status(&store)? {
                Some(state) => {
                    println!("Active session: {}", state.id);
                    println!("Project: {}", state.project_display_name);
                    println!("Tool: {}", state.tool);
                    if let Some(topic) = state.topic {
                        println!("Topic: {}", topic);
                    }
                    println!("Started: {}", state.start_time_utc);
                    println!("Transcript: {}", state.transcript_path.display());
                }
                None => {
                    println!("No active session.");
                }
            }
            Ok(())
        }
    }
}

fn handle_placeholder<T>(area: &str, _cmd: T) -> Result<()> {
    println!("{area} commands are not implemented yet.");
    Ok(())
}

fn handle_note(cmd: NoteCommands, config_path: Option<&Path>) -> Result<()> {
    let config_path = resolve_config_path(config_path)?;
    let config = aiw_config::Config::load(&config_path)?;
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
            for cmd in &commands {
                let prompt = build_note_prompt(&cmd.raw, &content);
                let output = aiw_ai_tools::run_prompt(&adapter, &prompt)?;
                let block = format_note_result_block(&cmd.raw, &output.stdout, &output.stderr);
                appended_blocks.push_str(&block);
            }

            let mut updated = content;
            if !updated.ends_with('\n') {
                updated.push('\n');
            }
            updated.push_str(&appended_blocks);
            fs::write(&resolved, updated)
                .with_context(|| format!("Failed to write note {}", resolved.display()))?;
            println!("Processed {} command(s) in {}", commands.len(), resolved.display());
            Ok(())
        }
    }
}

fn handle_adr(cmd: AdrCommands, config_path: Option<&Path>) -> Result<()> {
    let config_path = resolve_config_path(config_path)?;
    let config = aiw_config::Config::load(&config_path)?;
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

fn resolve_config_path(config_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = config_path {
        return Ok(path.to_path_buf());
    }
    if let Ok(path) = std::env::var("AIW_CONFIG") {
        return Ok(PathBuf::from(path));
    }
    Ok(PathBuf::from(DEFAULT_CONFIG_FILE))
}

fn session_state_dir(config_path: &Path) -> Result<PathBuf> {
    let base = config_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    Ok(base.join(".aiw"))
}

fn prompt_dev_log_input() -> Result<aiw_session::DevLogInput> {
    println!("Enter session details for the dev log. Leave blank if not applicable.");
    let goal = prompt_line("Goal")?;
    let summary = prompt_line("Summary")?;
    let decision = prompt_line("Decision")?;
    let rationale = prompt_line("Rationale")?;
    let follow_up_tasks = prompt_line("Follow-up tasks")?;

    Ok(aiw_session::DevLogInput {
        goal,
        summary,
        decision,
        rationale,
        follow_up_tasks,
    })
}

fn prompt_line(label: &str) -> Result<String> {
    use std::io::{self, Write};

    print!("{label}: ");
    io::stdout().flush().context("Failed to flush stdout")?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("Failed to read input")?;
    Ok(input.trim().to_string())
}

fn prompt_adr_input(title: Option<String>) -> Result<aiw_adr::AdrInput> {
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

fn prompt_yes_no(label: &str) -> Result<bool> {
    let response = prompt_line(label)?;
    let response = response.trim().to_ascii_lowercase();
    Ok(matches!(response.as_str(), "y" | "yes"))
}

fn build_note_prompt(command: &str, content: &str) -> String {
    format!(
        "You are assisting with a markdown note.\nCommand: {command}\n\nNote content:\n{content}\n"
    )
}

fn format_note_result_block(command: &str, stdout: &str, stderr: &str) -> String {
    let mut block = String::new();
    block.push_str("\n---\n");
    block.push_str(&format!("**AI Result ({command})**\n\n"));
    if !stdout.trim().is_empty() {
        block.push_str("```text\n");
        block.push_str(stdout.trim());
        block.push_str("\n```\n");
    }
    if !stderr.trim().is_empty() {
        block.push_str("\n**Tool Stderr**\n\n```text\n");
        block.push_str(stderr.trim());
        block.push_str("\n```\n");
    }
    block
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
    let sample = include_str!("../../../config/ai-dev-workflow.example.toml");
    fs::write(path, sample).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}
