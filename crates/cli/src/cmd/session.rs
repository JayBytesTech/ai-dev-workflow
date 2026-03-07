use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{OutputFormat, SessionCommands, SessionDoctorArgs, SessionEndArgs};

use super::{
    prompt_line, prompt_line_with_default, prompt_yes_no, resolve_config_path, session_state_dir,
};
use crate::cmd::adr::{prompt_adr_input, prompt_adr_input_with_defaults};

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub(crate) struct SessionEndOutput {
    pub(crate) session_id: String,
    pub(crate) project: String,
    pub(crate) tool: String,
    pub(crate) capture_status: String,
    pub(crate) transcript_path: String,
    pub(crate) dev_log_path: String,
    pub(crate) adr_path: Option<String>,
}

#[derive(Deserialize)]
struct AutoDevLogFields {
    summary: String,
    decision: String,
    rationale: String,
    follow_up_tasks: Vec<String>,
}

#[derive(Deserialize)]
struct AutoAdrFields {
    title: String,
    context: String,
    options: String,
    decision: String,
    consequences: String,
}

// ---------------------------------------------------------------------------
// Doctor helpers
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum DoctorLevel {
    Ok,
    Warn,
    Error,
}

struct DoctorReport {
    ok: usize,
    warn: usize,
    error: usize,
}

impl DoctorReport {
    fn new() -> Self {
        Self {
            ok: 0,
            warn: 0,
            error: 0,
        }
    }

    fn add(&mut self, level: DoctorLevel, message: impl AsRef<str>) {
        match level {
            DoctorLevel::Ok => {
                self.ok += 1;
                println!("[ok] {}", message.as_ref());
            }
            DoctorLevel::Warn => {
                self.warn += 1;
                println!("[warn] {}", message.as_ref());
            }
            DoctorLevel::Error => {
                self.error += 1;
                println!("[error] {}", message.as_ref());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Main handler
// ---------------------------------------------------------------------------

pub(crate) fn handle_session(
    cmd: SessionCommands,
    config_path: Option<&Path>,
    profile: Option<&str>,
) -> Result<()> {
    let config_path = resolve_config_path(config_path)?;
    let state_dir = session_state_dir(&config_path, profile);
    let store = aiw_session::SessionStore::new(&state_dir)?;

    match cmd {
        SessionCommands::Start(args) => {
            let config = aiw_config::Config::load_with_profile(&config_path, profile)?;
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
                let _ = aiw_session::update_capture_status(
                    &store,
                    aiw_session::TranscriptCaptureStatus::Capturing,
                )?;
                let tool_kind = aiw_ai_tools::ToolKind::parse(&args.tool)?;
                let adapter = aiw_ai_tools::ToolAdapter::from_config(&config, tool_kind)?;
                println!("Launching tool: {}", adapter.executable);
                let prefer_script = if args.no_script {
                    false
                } else if args.script {
                    true
                } else {
                    !matches!(tool_kind, aiw_ai_tools::ToolKind::Codex)
                };
                let code = aiw_session::run_tool_with_transcript(
                    &adapter.executable,
                    &args.tool_args,
                    &state.transcript_path,
                    args.pty,
                    prefer_script,
                    aiw_session::PtyConfig {
                        cols: args.pty_cols,
                        rows: args.pty_rows,
                    },
                )?;
                let _ = aiw_session::refresh_capture_checkpoint(&store)?;
                let status = if code == 0 {
                    aiw_session::TranscriptCaptureStatus::Flushed
                } else {
                    aiw_session::TranscriptCaptureStatus::Failed
                };
                let _ = aiw_session::update_capture_status(&store, status)?;
                println!("Tool exited with code {code}");
            }
            Ok(())
        }
        SessionCommands::End(args) => {
            let config = aiw_config::Config::load_with_profile(&config_path, profile)?;
            let report = config.validate();
            if !report.is_ok() {
                println!("{report}");
                return Err(anyhow::anyhow!("config validation failed"));
            }
            let state = aiw_session::end_session(&store)?;
            if let Err(err) = aiw_session::cleanup_transcript(&state.transcript_path) {
                eprintln!(
                    "[aiw] transcript cleanup skipped ({}): {err}",
                    state.transcript_path.display()
                );
            }
            let project = config
                .projects
                .get(&state.project_key)
                .ok_or_else(|| anyhow::anyhow!("project not found: {}", state.project_key))?;

            if args.no_adr
                && (args.adr_title.is_some()
                    || args.adr_context.is_some()
                    || args.adr_options.is_some()
                    || args.adr_decision.is_some()
                    || args.adr_consequences.is_some()
                    || args.auto_adr)
            {
                return Err(anyhow::anyhow!(
                    "ADR flags cannot be used together with --no-adr"
                ));
            }

            let input = build_session_end_input(&config, &state, &args)?;
            let git_info = aiw_session::collect_git_info(project);
            let dev_log_path =
                aiw_session::write_dev_log(&config, project, &state, input, git_info)?;

            let adr_path = maybe_create_session_adr(&config, project, &state, &args)?;
            emit_session_end_output(
                args.output,
                SessionEndOutput {
                    session_id: state.id,
                    project: state.project_display_name,
                    tool: state.tool,
                    capture_status: state.capture_status.to_string(),
                    transcript_path: state.transcript_path.display().to_string(),
                    dev_log_path: dev_log_path.display().to_string(),
                    adr_path: adr_path.map(|path| path.display().to_string()),
                },
            )?;
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
                    println!("Capture status: {}", state.capture_status);
                    println!("Transcript bytes: {}", state.last_transcript_size_bytes);
                }
                None => {
                    println!("No active session.");
                }
            }
            Ok(())
        }
        SessionCommands::Doctor(args) => {
            let config = aiw_config::Config::load_with_profile(&config_path, profile)?;
            run_session_doctor(&config, &config_path, &state_dir, &args)
        }
    }
}

// ---------------------------------------------------------------------------
// Doctor
// ---------------------------------------------------------------------------

fn run_session_doctor(
    config: &aiw_config::Config,
    config_path: &Path,
    state_dir: &Path,
    args: &SessionDoctorArgs,
) -> Result<()> {
    println!("Session doctor report");
    println!("Config: {}", config_path.display());
    println!("Project: {}", args.project);
    println!("Tool: {}", args.tool);
    println!();

    let mut report = DoctorReport::new();
    let validation = config.validate();
    if validation.is_ok() {
        report.add(DoctorLevel::Ok, "config validation passed");
    } else {
        for err in validation.errors {
            report.add(DoctorLevel::Error, format!("config error: {err}"));
        }
    }
    for warning in validation.warnings {
        report.add(DoctorLevel::Warn, format!("config warning: {warning}"));
    }

    match config.projects.get(&args.project) {
        Some(project) => {
            report.add(
                DoctorLevel::Ok,
                format!("project exists: {}", project.display_name),
            );
            check_project_paths(config, project, &mut report);
            check_template(config, &mut report);
        }
        None => {
            report.add(
                DoctorLevel::Error,
                format!("project not found in config: {}", args.project),
            );
        }
    }

    check_tool(config, &args.tool, &mut report);
    check_session_state_dir(state_dir, &mut report);
    check_active_session_health(state_dir, args.repair, &mut report)?;
    check_binary_freshness(&mut report);

    println!();
    println!(
        "Doctor summary: {} ok, {} warning(s), {} error(s)",
        report.ok, report.warn, report.error
    );

    if report.error > 0 {
        return Err(anyhow::anyhow!("session doctor found errors"));
    }
    Ok(())
}

fn check_active_session_health(
    state_dir: &Path,
    repair: bool,
    report: &mut DoctorReport,
) -> Result<()> {
    let store = aiw_session::SessionStore::new(state_dir)?;
    let Some(state) = store.load()? else {
        report.add(DoctorLevel::Ok, "no active session state found");
        return Ok(());
    };

    report.add(
        DoctorLevel::Warn,
        format!(
            "active session present: {} ({})",
            state.id, state.capture_status
        ),
    );

    if state.capture_status != aiw_session::TranscriptCaptureStatus::Capturing {
        report.add(
            DoctorLevel::Ok,
            format!("capture state is terminal: {}", state.capture_status),
        );
        return Ok(());
    }

    if state.transcript_path.exists() {
        let bytes = fs::metadata(&state.transcript_path)
            .map(|m| m.len())
            .unwrap_or(0);
        report.add(
            DoctorLevel::Warn,
            format!(
                "capture is still marked capturing (transcript exists, {bytes} bytes): {}",
                state.transcript_path.display()
            ),
        );
    } else {
        report.add(
            DoctorLevel::Error,
            format!(
                "capture is marked capturing but transcript is missing: {}",
                state.transcript_path.display()
            ),
        );
    }

    if repair {
        let repaired = aiw_session::recover_active_session(&store)?;
        if let Some(updated) = repaired {
            report.add(
                DoctorLevel::Ok,
                format!(
                    "repair applied: capture state is now {}",
                    updated.capture_status
                ),
            );
        }
    } else {
        report.add(
            DoctorLevel::Warn,
            "run session doctor with --repair to repair stale capture state",
        );
    }
    Ok(())
}

fn check_project_paths(
    config: &aiw_config::Config,
    project: &aiw_config::ProjectConfig,
    report: &mut DoctorReport,
) {
    let transcript_dir =
        match aiw_config::resolve_in_vault(&config.vault_path, &project.transcript_dir) {
            Ok(path) => path,
            Err(err) => {
                report.add(DoctorLevel::Error, format!("invalid transcript_dir: {err}"));
                return;
            }
        };
    if let Err(err) = fs::create_dir_all(&transcript_dir) {
        report.add(
            DoctorLevel::Error,
            format!(
                "cannot create transcript_dir {}: {err}",
                transcript_dir.display()
            ),
        );
    } else {
        report.add(
            DoctorLevel::Ok,
            format!("transcript_dir accessible: {}", transcript_dir.display()),
        );
    }

    let dev_log_dir = match aiw_config::resolve_in_vault(&config.vault_path, &project.dev_logs_dir)
    {
        Ok(path) => path,
        Err(err) => {
            report.add(DoctorLevel::Error, format!("invalid dev_logs_dir: {err}"));
            return;
        }
    };
    if let Err(err) = fs::create_dir_all(&dev_log_dir) {
        report.add(
            DoctorLevel::Error,
            format!(
                "cannot create dev_logs_dir {}: {err}",
                dev_log_dir.display()
            ),
        );
    } else {
        report.add(
            DoctorLevel::Ok,
            format!("dev_logs_dir accessible: {}", dev_log_dir.display()),
        );
    }
}

fn check_template(config: &aiw_config::Config, report: &mut DoctorReport) {
    let templates_root =
        match aiw_config::resolve_in_vault(&config.vault_path, &config.templates_dir) {
            Ok(path) => path,
            Err(err) => {
                report.add(DoctorLevel::Error, format!("invalid templates_dir: {err}"));
                return;
            }
        };
    let template_path = templates_root.join(&config.dev_log_template);
    let template = match fs::read_to_string(&template_path) {
        Ok(raw) => raw,
        Err(err) => {
            report.add(
                DoctorLevel::Error,
                format!(
                    "cannot read dev_log_template {}: {err}",
                    template_path.display()
                ),
            );
            return;
        }
    };
    report.add(
        DoctorLevel::Ok,
        format!("dev_log_template loaded: {}", template_path.display()),
    );

    let required = [
        "{{summary}}",
        "{{decision}}",
        "{{rationale}}",
        "{{follow_up_tasks}}",
        "{{transcript_link}}",
        "{{transcript_excerpt}}",
    ];
    for placeholder in required {
        if template.contains(placeholder) {
            report.add(
                DoctorLevel::Ok,
                format!("template placeholder present: {placeholder}"),
            );
        } else {
            report.add(
                DoctorLevel::Warn,
                format!("template placeholder missing: {placeholder}"),
            );
        }
    }
}

fn check_tool(config: &aiw_config::Config, tool: &str, report: &mut DoctorReport) {
    let kind = match aiw_ai_tools::ToolKind::parse(tool) {
        Ok(kind) => kind,
        Err(err) => {
            report.add(DoctorLevel::Error, format!("unsupported tool: {err}"));
            return;
        }
    };
    let adapter = match aiw_ai_tools::ToolAdapter::from_config(config, kind) {
        Ok(adapter) => adapter,
        Err(err) => {
            report.add(DoctorLevel::Error, format!("tool config error: {err}"));
            return;
        }
    };
    report.add(
        DoctorLevel::Ok,
        format!("tool executable configured: {}", adapter.executable),
    );

    if executable_is_available(&adapter.executable) {
        report.add(
            DoctorLevel::Ok,
            format!(
                "tool executable found on PATH or filesystem: {}",
                adapter.executable
            ),
        );
    } else {
        report.add(
            DoctorLevel::Error,
            format!("tool executable not found: {}", adapter.executable),
        );
    }
}

fn executable_is_available(executable: &str) -> bool {
    let path = Path::new(executable);
    if path.components().count() > 1 {
        return path.exists();
    }
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| dir.join(executable).exists())
}

fn check_session_state_dir(state_dir: &Path, report: &mut DoctorReport) {
    if let Err(err) = fs::create_dir_all(state_dir) {
        report.add(
            DoctorLevel::Error,
            format!("cannot create state dir {}: {err}", state_dir.display()),
        );
        return;
    }
    let probe = state_dir.join("doctor-write-test.tmp");
    match fs::write(&probe, b"ok") {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            report.add(
                DoctorLevel::Ok,
                format!("state dir writable: {}", state_dir.display()),
            );
        }
        Err(err) => {
            report.add(
                DoctorLevel::Error,
                format!("state dir not writable {}: {err}", state_dir.display()),
            );
        }
    }
}

fn check_binary_freshness(report: &mut DoctorReport) {
    let current = match std::env::current_exe() {
        Ok(path) => path,
        Err(err) => {
            report.add(
                DoctorLevel::Warn,
                format!("cannot resolve current executable path: {err}"),
            );
            return;
        }
    };
    report.add(
        DoctorLevel::Ok,
        format!("current aiw binary: {}", current.display()),
    );

    let debug = PathBuf::from("target/debug/aiw");
    if !debug.exists() {
        return;
    }

    let current_mtime = fs::metadata(&current)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let debug_mtime = fs::metadata(&debug)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    if current != debug && debug_mtime > current_mtime {
        report.add(
            DoctorLevel::Warn,
            format!(
                "newer workspace binary detected at {}. If behavior differs, run this newer binary or reinstall.",
                debug.display()
            ),
        );
    }
}

// ---------------------------------------------------------------------------
// Session end: input building
// ---------------------------------------------------------------------------

fn build_session_end_input(
    config: &aiw_config::Config,
    state: &aiw_session::SessionState,
    args: &SessionEndArgs,
) -> Result<aiw_session::DevLogInput> {
    if args.auto_tool.is_some() && !(args.auto || args.auto_adr) {
        return Err(anyhow::anyhow!("--auto-tool requires --auto or --auto-adr"));
    }

    let default_goal = state.topic.clone().unwrap_or_default();
    let mut draft = None;
    if args.auto {
        let tool_kind = resolve_auto_tool_kind(args, state)?;
        if matches!(tool_kind, aiw_ai_tools::ToolKind::Codex) {
            eprintln!(
                "[aiw] auto-generation is not supported for codex; falling back to manual prompts"
            );
        } else {
            match generate_dev_log_input_from_transcript(config, state, tool_kind) {
                Ok(generated) => draft = Some(generated),
                Err(err) => {
                    if args.non_interactive {
                        return Err(err.context("Auto-generation failed in non-interactive mode"));
                    }
                    eprintln!(
                        "[aiw] auto-generation failed, falling back to manual prompts: {err}"
                    );
                }
            }
        }
    }

    if args.non_interactive {
        let mut input = draft.unwrap_or(aiw_session::DevLogInput {
            goal: default_goal.clone(),
            summary: String::new(),
            decision: String::new(),
            rationale: String::new(),
            follow_up_tasks: String::new(),
        });
        if let Some(goal) = &args.goal {
            input.goal = goal.clone();
        } else if input.goal.trim().is_empty() {
            input.goal = default_goal;
        }
        if let Some(summary) = &args.summary {
            input.summary = summary.clone();
        }
        if let Some(decision) = &args.decision {
            input.decision = decision.clone();
        }
        if let Some(rationale) = &args.rationale {
            input.rationale = rationale.clone();
        }
        if !args.follow_up_task.is_empty() {
            input.follow_up_tasks = format_follow_up_tasks(&args.follow_up_task);
        }
        return Ok(input);
    }

    if let Some(draft) = draft {
        println!("Generated draft from transcript. Press Enter to keep a suggested value.");
        return prompt_dev_log_input_with_defaults(&draft);
    }
    prompt_dev_log_input()
}

fn maybe_create_session_adr(
    config: &aiw_config::Config,
    project: &aiw_config::ProjectConfig,
    state: &aiw_session::SessionState,
    args: &SessionEndArgs,
) -> Result<Option<PathBuf>> {
    if args.no_adr {
        return Ok(None);
    }

    if let Some(input) = adr_input_from_flags(args)? {
        let path = aiw_adr::create_adr(config, project, input)?;
        return Ok(Some(path));
    }

    if args.auto_adr {
        let tool_kind = resolve_auto_tool_kind(args, state)?;
        if matches!(tool_kind, aiw_ai_tools::ToolKind::Codex) {
            if args.non_interactive {
                return Err(anyhow::anyhow!(
                    "Auto-ADR generation is not supported for codex. Use --auto-tool to select another tool."
                ));
            }
            eprintln!(
                "[aiw] auto-generation is not supported for codex; falling back to manual prompts"
            );
            let adr_input = prompt_adr_input(None)?;
            let path = aiw_adr::create_adr(config, project, adr_input)?;
            return Ok(Some(path));
        }
        match generate_adr_input_from_transcript(config, state, tool_kind) {
            Ok(generated) => {
                let adr_input = if args.non_interactive {
                    generated
                } else {
                    println!(
                        "Generated ADR draft from transcript. Press Enter to keep a suggested value."
                    );
                    prompt_adr_input_with_defaults(&generated)?
                };
                let path = aiw_adr::create_adr(config, project, adr_input)?;
                return Ok(Some(path));
            }
            Err(err) => {
                if args.non_interactive {
                    return Err(err.context("Auto-ADR generation failed in non-interactive mode"));
                }
                eprintln!("[aiw] auto-generation failed, falling back to manual prompts: {err}");
                let adr_input = prompt_adr_input(None)?;
                let path = aiw_adr::create_adr(config, project, adr_input)?;
                return Ok(Some(path));
            }
        }
    }

    if args.non_interactive {
        return Ok(None);
    }

    if prompt_yes_no("Create ADR? (y/N)")? {
        let adr_input = prompt_adr_input(None)?;
        let path = aiw_adr::create_adr(config, project, adr_input)?;
        return Ok(Some(path));
    }

    Ok(None)
}

fn adr_input_from_flags(args: &SessionEndArgs) -> Result<Option<aiw_adr::AdrInput>> {
    let any_adr_flags = args.adr_title.is_some()
        || args.adr_context.is_some()
        || args.adr_options.is_some()
        || args.adr_decision.is_some()
        || args.adr_consequences.is_some();
    if !any_adr_flags {
        return Ok(None);
    }

    let title = args
        .adr_title
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--adr-title is required when using ADR flags"))?;
    let context = args
        .adr_context
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--adr-context is required when using ADR flags"))?;
    let options = args
        .adr_options
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--adr-options is required when using ADR flags"))?;
    let decision = args
        .adr_decision
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--adr-decision is required when using ADR flags"))?;
    let consequences = args
        .adr_consequences
        .clone()
        .ok_or_else(|| anyhow::anyhow!("--adr-consequences is required when using ADR flags"))?;

    Ok(Some(aiw_adr::AdrInput {
        title,
        context,
        options,
        decision,
        consequences,
    }))
}

fn emit_session_end_output(format: OutputFormat, output: SessionEndOutput) -> Result<()> {
    match format {
        OutputFormat::Text => {
            println!("Ended session: {}", output.session_id);
            println!("Project: {}", output.project);
            println!("Tool: {}", output.tool);
            println!("Capture status: {}", output.capture_status);
            println!("Transcript: {}", output.transcript_path);
            println!("Created dev log: {}", output.dev_log_path);
            if let Some(adr_path) = output.adr_path {
                println!("Created ADR: {}", adr_path);
            }
        }
        OutputFormat::Json => {
            let rendered =
                serde_json::to_string_pretty(&output).context("Failed to serialize JSON output")?;
            println!("{rendered}");
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Dev log prompts
// ---------------------------------------------------------------------------

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

fn prompt_dev_log_input_with_defaults(
    defaults: &aiw_session::DevLogInput,
) -> Result<aiw_session::DevLogInput> {
    println!("Enter session details for the dev log. Leave blank to keep suggested values.");
    let goal = prompt_line_with_default("Goal", &defaults.goal)?;
    let summary = prompt_line_with_default("Summary", &defaults.summary)?;
    let decision = prompt_line_with_default("Decision", &defaults.decision)?;
    let rationale = prompt_line_with_default("Rationale", &defaults.rationale)?;
    let follow_up_tasks = prompt_line_with_default("Follow-up tasks", &defaults.follow_up_tasks)?;

    Ok(aiw_session::DevLogInput {
        goal,
        summary,
        decision,
        rationale,
        follow_up_tasks,
    })
}

// ---------------------------------------------------------------------------
// Auto-generation
// ---------------------------------------------------------------------------

fn resolve_auto_tool_kind(
    args: &SessionEndArgs,
    state: &aiw_session::SessionState,
) -> Result<aiw_ai_tools::ToolKind> {
    match &args.auto_tool {
        Some(tool) => aiw_ai_tools::ToolKind::parse(tool),
        None => aiw_ai_tools::ToolKind::parse(&state.tool),
    }
}

fn generate_dev_log_input_from_transcript(
    config: &aiw_config::Config,
    state: &aiw_session::SessionState,
    tool_kind: aiw_ai_tools::ToolKind,
) -> Result<aiw_session::DevLogInput> {
    let transcript = read_transcript_tail(&state.transcript_path, 12_000)?;
    let adapter = aiw_ai_tools::ToolAdapter::from_config(config, tool_kind)?;
    let prompt = build_session_end_auto_prompt(state, &transcript);
    let output = aiw_ai_tools::run_prompt(&adapter, &prompt)?;

    let json = extract_json_block(&output.stdout).unwrap_or(output.stdout.as_str());
    let fields: AutoDevLogFields = serde_json::from_str(json).with_context(|| {
        format!(
            "Failed to parse auto-generated JSON from tool output. Raw output:\n{}",
            output.stdout
        )
    })?;

    Ok(aiw_session::DevLogInput {
        goal: state.topic.clone().unwrap_or_default(),
        summary: fields.summary.trim().to_string(),
        decision: fields.decision.trim().to_string(),
        rationale: fields.rationale.trim().to_string(),
        follow_up_tasks: format_follow_up_tasks(&fields.follow_up_tasks),
    })
}

fn generate_adr_input_from_transcript(
    config: &aiw_config::Config,
    state: &aiw_session::SessionState,
    tool_kind: aiw_ai_tools::ToolKind,
) -> Result<aiw_adr::AdrInput> {
    let transcript = read_transcript_tail(&state.transcript_path, 12_000)?;
    let adapter = aiw_ai_tools::ToolAdapter::from_config(config, tool_kind)?;
    let prompt = build_session_end_auto_adr_prompt(state, &transcript);
    let output = aiw_ai_tools::run_prompt(&adapter, &prompt)?;

    let json = extract_json_block(&output.stdout).unwrap_or(output.stdout.as_str());
    let fields: AutoAdrFields = serde_json::from_str(json).with_context(|| {
        format!(
            "Failed to parse auto-generated JSON from tool output. Raw output:\n{}",
            output.stdout
        )
    })?;

    Ok(aiw_adr::AdrInput {
        title: fields.title.trim().to_string(),
        context: fields.context.trim().to_string(),
        options: fields.options.trim().to_string(),
        decision: fields.decision.trim().to_string(),
        consequences: fields.consequences.trim().to_string(),
    })
}

fn read_transcript_tail(path: &Path, max_chars: usize) -> Result<String> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read transcript {}", path.display()))?;
    let total_chars = content.chars().count();
    if total_chars <= max_chars {
        return Ok(content);
    }
    let skip = total_chars.saturating_sub(max_chars);
    Ok(content.chars().skip(skip).collect())
}

fn build_session_end_auto_prompt(
    state: &aiw_session::SessionState,
    transcript_tail: &str,
) -> String {
    format!(
        "You generate a concise dev-log draft from a terminal transcript.\n\
Return STRICT JSON only (no markdown, no code fences) with exactly these keys:\n\
summary (string), decision (string), rationale (string), follow_up_tasks (array of strings).\n\
Rules:\n\
- summary: 1-3 sentences focused on meaningful outcomes.\n\
- decision: concrete decisions made in this session.\n\
- rationale: why those decisions were chosen.\n\
- follow_up_tasks: 1-6 actionable tasks.\n\
- If unavailable, use empty string/empty array.\n\
\n\
Session:\n\
project={}\n\
tool={}\n\
topic={}\n\
\n\
Transcript tail:\n\
{}\n",
        state.project_display_name,
        state.tool,
        state.topic.clone().unwrap_or_else(|| "N/A".to_string()),
        transcript_tail
    )
}

fn build_session_end_auto_adr_prompt(
    state: &aiw_session::SessionState,
    transcript_tail: &str,
) -> String {
    format!(
        "You generate an ADR draft from a terminal transcript.\n\
Return STRICT JSON only (no markdown, no code fences) with exactly these keys:\n\
title (string), context (string), options (string), decision (string), consequences (string).\n\
Rules:\n\
- title: short, concrete decision title.\n\
- context: 2-6 sentences summarizing the situation.\n\
- options: bullet list as a single string (use '-' lines).\n\
- decision: 1-3 sentences describing the chosen option.\n\
- consequences: 1-4 sentences describing tradeoffs or follow-ups.\n\
- If unavailable, use empty string.\n\
\n\
Session:\n\
project={}\n\
tool={}\n\
topic={}\n\
\n\
Transcript tail:\n\
{}\n",
        state.project_display_name,
        state.tool,
        state.topic.clone().unwrap_or_else(|| "N/A".to_string()),
        transcript_tail
    )
}

fn extract_json_block(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end < start {
        return None;
    }
    Some(&text[start..=end])
}

fn format_follow_up_tasks(tasks: &[String]) -> String {
    let lines: Vec<String> = tasks
        .iter()
        .map(|task| task.trim())
        .filter(|task| !task.is_empty())
        .map(|task| format!("- [ ] {task}"))
        .collect();
    lines.join("\n")
}
