use std::io;
#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};

mod cmd;

// ---------------------------------------------------------------------------
// CLI type definitions
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "aiw", version, about = "AI development workflow helper")]
struct Cli {
    #[arg(
        long,
        value_name = "path",
        help = "Path to the config file (default: ./ai-dev-workflow.toml or AIW_CONFIG)"
    )]
    config: Option<PathBuf>,
    #[arg(
        long,
        value_name = "name",
        help = "Config profile to activate (overrides defaults with [profiles.<name>])"
    )]
    profile: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(subcommand, about = "Manage the aiw config file")]
    Config(ConfigCommands),
    #[command(subcommand, about = "Start, end, and inspect AI sessions")]
    Session(SessionCommands),
    #[command(subcommand, about = "Scan and process Obsidian notes with AI commands")]
    Note(NoteCommands),
    #[command(subcommand, about = "Create Architecture Decision Records")]
    Adr(AdrCommands),
    #[command(subcommand, about = "List configured projects")]
    Projects(ProjectsCommands),
    #[command(about = "Search the vault for a keyword or phrase")]
    Search(SearchArgs),
    #[command(about = "Print shell completion script for the given shell")]
    Completions(CompletionsArgs),
}

#[derive(Subcommand)]
enum ConfigCommands {
    #[command(about = "Write a starter config file")]
    Init(ConfigInitArgs),
    #[command(about = "Validate the config file and report errors")]
    Validate(ConfigValidateArgs),
    #[command(about = "Print the active config (optionally fully resolved)")]
    Show(ConfigShowArgs),
}

#[derive(Args)]
struct ConfigInitArgs {
    #[arg(
        long,
        value_name = "path",
        help = "Where to write the config file (default: ./ai-dev-workflow.toml)"
    )]
    output: Option<PathBuf>,
}

#[derive(Args)]
struct ConfigValidateArgs {}

#[derive(Args)]
struct ConfigShowArgs {
    #[arg(
        long,
        help = "Show the fully resolved config after profile/env overrides"
    )]
    resolved: bool,
}

#[derive(Subcommand)]
enum SessionCommands {
    #[command(about = "Begin a new session for a project and tool")]
    Start(SessionStartArgs),
    #[command(about = "End the active session and write a dev log to the vault")]
    End(SessionEndArgs),
    #[command(about = "Show the status of the active session")]
    Status,
    #[command(about = "Inspect or repair a stale session state")]
    Doctor(SessionDoctorArgs),
}

#[derive(Args)]
struct SessionStartArgs {
    #[arg(
        long,
        value_name = "name",
        help = "Project key from your config (see `aiw projects list`)"
    )]
    project: String,
    #[arg(
        long,
        value_name = "tool",
        help = "AI tool to use: claude, gemini, or codex"
    )]
    tool: String,
    #[arg(
        long,
        value_name = "text",
        help = "Short description of what you are working on"
    )]
    topic: Option<String>,
    #[arg(
        long,
        value_name = "path",
        help = "Working directory to record in the session (default: current dir)"
    )]
    cwd: Option<PathBuf>,
    #[arg(
        long,
        help = "Run the tool and capture stdout/stderr to the transcript file"
    )]
    wrap: bool,
    #[arg(long, value_name = "arg", num_args = 1.., help = "Arguments to pass to the tool when using --wrap")]
    tool_args: Vec<String>,
    #[arg(
        long,
        help = "Use a PTY for richer transcript capture when wrapping a tool"
    )]
    pty: bool,
    #[arg(
        long,
        help = "Disable the script(1) backend when using --pty (force native PTY)"
    )]
    no_script: bool,
    #[arg(long, help = "Force the script(1) backend when using --pty")]
    script: bool,
    #[arg(long, value_name = "cols", default_value_t = default_pty_cols(), help = "PTY columns when using --pty")]
    pty_cols: u16,
    #[arg(long, value_name = "rows", default_value_t = default_pty_rows(), help = "PTY rows when using --pty")]
    pty_rows: u16,
}

#[derive(Args)]
struct SessionEndArgs {
    #[arg(
        long,
        help = "Generate dev-log fields from transcript and edit defaults"
    )]
    auto: bool,
    #[arg(long, help = "Auto-generate ADR fields from transcript")]
    auto_adr: bool,
    #[arg(
        long = "auto-tool",
        value_name = "tool",
        help = "Tool to use for auto-generation (overrides session tool)"
    )]
    auto_tool: Option<String>,
    #[arg(
        long,
        help = "Do not prompt for input; use flags and/or auto-generated values"
    )]
    non_interactive: bool,
    #[arg(long, value_name = "text", help = "Goal or intent for the session")]
    goal: Option<String>,
    #[arg(long, value_name = "text", help = "Summary of what was accomplished")]
    summary: Option<String>,
    #[arg(
        long,
        value_name = "text",
        help = "Key decision made during the session"
    )]
    decision: Option<String>,
    #[arg(long, value_name = "text", help = "Rationale behind the decision")]
    rationale: Option<String>,
    #[arg(
        long = "follow-up-task",
        value_name = "text",
        help = "Follow-up task to record (repeatable)"
    )]
    follow_up_task: Vec<String>,
    #[arg(long, help = "Skip ADR creation entirely")]
    no_adr: bool,
    #[arg(long, value_name = "text", help = "Title for the ADR")]
    adr_title: Option<String>,
    #[arg(long, value_name = "text", help = "Context section for the ADR")]
    adr_context: Option<String>,
    #[arg(long, value_name = "text", help = "Options considered in the ADR")]
    adr_options: Option<String>,
    #[arg(long, value_name = "text", help = "Decision recorded in the ADR")]
    adr_decision: Option<String>,
    #[arg(long, value_name = "text", help = "Consequences of the ADR decision")]
    adr_consequences: Option<String>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text, help = "Output format for machine-readable results")]
    output: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Args)]
struct SessionDoctorArgs {
    #[arg(long, value_name = "name", help = "Project key to inspect")]
    project: String,
    #[arg(
        long,
        value_name = "tool",
        default_value = "codex",
        help = "Tool whose session state to inspect"
    )]
    tool: String,
    #[arg(
        long,
        help = "Repair stale active sessions with unfinished transcript capture state"
    )]
    repair: bool,
}

#[derive(Subcommand)]
enum NoteCommands {
    #[command(about = "Scan a note for /ai commands and report what was found")]
    Scan(NoteScanArgs),
    #[command(about = "Execute /ai commands in a note and append AI-generated results")]
    Process(NoteProcessArgs),
}

#[derive(Args)]
struct NoteScanArgs {
    #[arg(
        long,
        value_name = "name",
        help = "Project key (determines allowed folders and vault path)"
    )]
    project: String,
    #[arg(
        long,
        value_name = "path",
        help = "Vault-relative path to the note file"
    )]
    path: PathBuf,
}

#[derive(Args)]
struct NoteProcessArgs {
    #[arg(
        long,
        value_name = "name",
        help = "Project key (determines allowed folders and vault path)"
    )]
    project: String,
    #[arg(
        long,
        value_name = "tool",
        help = "AI tool to use for processing: claude, gemini, or codex"
    )]
    tool: String,
    #[arg(
        long,
        value_name = "path",
        help = "Vault-relative path to the note file"
    )]
    path: PathBuf,
}

#[derive(Subcommand)]
enum AdrCommands {
    #[command(about = "Create a new Architecture Decision Record in the vault")]
    Create(AdrCreateArgs),
}

#[derive(Args)]
struct AdrCreateArgs {
    #[arg(long, value_name = "name", help = "Project key the ADR belongs to")]
    project: String,
    #[arg(
        long,
        value_name = "text",
        help = "Short title for the ADR (used in the filename)"
    )]
    title: String,
}

#[derive(Subcommand)]
enum ProjectsCommands {
    #[command(about = "List all projects defined in the config")]
    List,
}

#[derive(Args)]
struct SearchArgs {
    #[arg(help = "Keyword or phrase to search for")]
    query: String,
    #[arg(long, help = "Restrict to a specific project (default: all projects)")]
    project: Option<String>,
    #[arg(
        long = "type",
        value_enum,
        default_value = "all",
        help = "Content type to search"
    )]
    content_type: ContentTypeArg,
    #[arg(long, help = "Only return results from files dated within N days")]
    since: Option<u32>,
    #[arg(long, default_value = "2", help = "Lines of context around each match")]
    context: usize,
    #[arg(long, help = "Include raw transcripts in search")]
    include_transcripts: bool,
    #[arg(
        long = "folder",
        help = "Additional vault-relative folder to include (repeatable)"
    )]
    folders: Vec<String>,
}

#[derive(Args)]
struct CompletionsArgs {
    #[arg(value_enum, help = "Target shell")]
    shell: clap_complete::Shell,
}

#[derive(Clone, ValueEnum)]
enum ContentTypeArg {
    All,
    DevLogs,
    Adrs,
}

impl From<ContentTypeArg> for aiw_obsidian::ContentTypeFilter {
    fn from(arg: ContentTypeArg) -> Self {
        match arg {
            ContentTypeArg::All => aiw_obsidian::ContentTypeFilter::All,
            ContentTypeArg::DevLogs => aiw_obsidian::ContentTypeFilter::DevLogs,
            ContentTypeArg::Adrs => aiw_obsidian::ContentTypeFilter::Adrs,
        }
    }
}

fn default_pty_cols() -> u16 {
    detect_terminal_size().map(|(cols, _)| cols).unwrap_or(120)
}

fn default_pty_rows() -> u16 {
    detect_terminal_size().map(|(_, rows)| rows).unwrap_or(30)
}

fn detect_terminal_size() -> Option<(u16, u16)> {
    #[cfg(unix)]
    {
        let fd = io::stdin().as_raw_fd();
        if unsafe { libc::isatty(fd) } != 1 {
            return None;
        }

        let mut ws = libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        if unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) } != 0 {
            return None;
        }
        if ws.ws_col == 0 || ws.ws_row == 0 {
            return None;
        }
        Some((ws.ws_col, ws.ws_row))
    }

    #[cfg(not(unix))]
    {
        None
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct WrappedToolExit(i32);

impl std::fmt::Display for WrappedToolExit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "wrapped tool exited with code {}", self.0)
    }
}

impl std::error::Error for WrappedToolExit {}

fn main() {
    match run() {
        Ok(()) => {}
        Err(err) => {
            if let Some(WrappedToolExit(code)) = err.downcast_ref::<WrappedToolExit>() {
                std::process::exit(*code);
            }
            emit_error_with_hints(&err);
            std::process::exit(1);
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let profile = cli.profile.as_deref();
    match cli.command {
        Commands::Config(cmd) => cmd::config::handle_config(cmd, cli.config.as_deref(), profile),
        Commands::Projects(cmd) => {
            cmd::projects::handle_projects(cmd, cli.config.as_deref(), profile)
        }
        Commands::Session(cmd) => cmd::session::handle_session(cmd, cli.config.as_deref(), profile),
        Commands::Note(cmd) => cmd::note::handle_note(cmd, cli.config.as_deref(), profile),
        Commands::Adr(cmd) => cmd::adr::handle_adr(cmd, cli.config.as_deref(), profile),
        Commands::Search(args) => cmd::search::handle_search(args, cli.config.as_deref(), profile),
        Commands::Completions(args) => {
            use clap::CommandFactory;
            use clap_complete::generate;
            let mut cmd = Cli::command();
            generate(args.shell, &mut cmd, "aiw", &mut std::io::stdout());
            Ok(())
        }
    }
}

fn emit_error_with_hints(err: &anyhow::Error) {
    eprintln!("Error: {err}");
    let mut chain = err.chain();
    let _ = chain.next();
    let mut has_causes = false;
    for cause in chain {
        if !has_causes {
            eprintln!("Caused by:");
            has_causes = true;
        }
        eprintln!("- {cause}");
    }

    let hints = error_hints(err);
    if !hints.is_empty() {
        for hint in hints {
            eprintln!("Hint: {hint}");
        }
    }
}

fn error_hints(err: &anyhow::Error) -> Vec<&'static str> {
    let mut hints = Vec::new();
    let mut push = |hint: &'static str| {
        if !hints.contains(&hint) {
            hints.push(hint);
        }
    };

    let messages: Vec<String> = err.chain().map(|e| e.to_string()).collect();
    let joined = messages.join(" | ");

    if joined.contains("Failed to read config at") {
        push("Create a config with `aiw config init` or pass `--config <path>`.");
    }
    if joined.contains("Failed to parse TOML config") {
        push("Fix TOML syntax and re-run `aiw config validate`.");
    }
    if joined.contains("profile not found in config") {
        push("Use `--profile <name>` that exists or add it under `[profiles]` in the config.");
    }
    if joined.contains("config validation failed") {
        push("Run `aiw config validate` to see detailed validation errors.");
    }
    if joined.contains("project not found:") {
        push("Run `aiw projects list` and choose a valid `--project` value.");
    }
    if joined.contains("unsupported tool:") {
        push("Supported tools: `claude`, `gemini`, `codex`.");
    }
    if joined.contains("tool executable is empty:") {
        push("Set the tool executable path in your config under `[tools]`.");
    }
    if joined.contains("Failed to spawn") {
        push("Verify the tool executable exists and is on your PATH (or set an absolute path).");
    }
    if joined.contains("note path is not within allowed folders") {
        push("Choose a note path under `allowed_note_folders` or update the config.");
    }
    if joined.contains("Failed to read note") {
        push("Confirm the note path exists under the project vault and is readable.");
    }
    if joined.contains("Failed to write note") {
        push("Confirm the note path is writable and not locked by another process.");
    }

    hints
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;

    #[test]
    fn error_hints_suggests_config_init() {
        let err = anyhow::anyhow!("Failed to read config at /tmp/missing.toml");
        let hints = error_hints(&err);
        assert!(hints.iter().any(|hint| hint.contains("config init")));
    }

    #[test]
    fn error_hints_suggests_projects_list() {
        let err = anyhow::anyhow!("project not found: demo");
        let hints = error_hints(&err);
        assert!(hints.iter().any(|hint| hint.contains("projects list")));
    }

    #[test]
    fn error_hints_suggests_tool_help_from_chain() {
        let err = anyhow::anyhow!("outer").context(anyhow::anyhow!("unsupported tool: demo"));
        let hints = error_hints(&err);
        assert!(hints.iter().any(|hint| hint.contains("Supported tools")));
    }
}
