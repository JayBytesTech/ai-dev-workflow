use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};

use aiw_config::{resolve_in_vault, Config, ProjectConfig};
use aiw_templates::{render_template, TemplateStore};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};

const STATE_FILE_NAME: &str = "session.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionState {
    pub id: String,
    pub project_key: String,
    pub project_display_name: String,
    pub tool: String,
    pub topic: Option<String>,
    pub start_time_utc: DateTime<Utc>,
    pub cwd: PathBuf,
    pub transcript_path: PathBuf,
}

pub struct SessionStore {
    state_path: PathBuf,
}

#[derive(Debug)]
pub struct DevLogInput {
    pub goal: String,
    pub summary: String,
    pub decision: String,
    pub rationale: String,
    pub follow_up_tasks: String,
}

#[derive(Debug, Default)]
pub struct GitInfo {
    pub files_changed: String,
    pub summary: String,
}

impl SessionStore {
    pub fn new(state_dir: impl AsRef<Path>) -> Result<Self> {
        let state_dir = state_dir.as_ref();
        if !state_dir.exists() {
            fs::create_dir_all(state_dir).with_context(|| {
                format!("Failed to create session state dir {}", state_dir.display())
            })?;
        }
        Ok(Self {
            state_path: state_dir.join(STATE_FILE_NAME),
        })
    }

    pub fn load(&self) -> Result<Option<SessionState>> {
        if !self.state_path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&self.state_path)
            .with_context(|| format!("Failed to read {}", self.state_path.display()))?;
        let state: SessionState = serde_json::from_str(&raw)
            .with_context(|| format!("Failed to parse {}", self.state_path.display()))?;
        Ok(Some(state))
    }

    pub fn save(&self, state: &SessionState) -> Result<()> {
        let raw = serde_json::to_string_pretty(state)
            .with_context(|| "Failed to serialize session state")?;
        fs::write(&self.state_path, raw)
            .with_context(|| format!("Failed to write {}", self.state_path.display()))?;
        Ok(())
    }

    pub fn clear(&self) -> Result<()> {
        if self.state_path.exists() {
            fs::remove_file(&self.state_path)
                .with_context(|| format!("Failed to remove {}", self.state_path.display()))?;
        }
        Ok(())
    }
}

pub fn start_session(
    config: &Config,
    project_key: &str,
    tool: &str,
    topic: Option<String>,
    cwd: PathBuf,
    store: &SessionStore,
) -> Result<SessionState> {
    let tool = normalize_tool(tool)?;
    if let Some(existing) = store.load()? {
        return Err(anyhow!(
            "an active session already exists (project: {}, tool: {})",
            existing.project_key,
            existing.tool
        ));
    }

    let project = resolve_project(config, project_key)?;
    let transcript_path = build_transcript_path(config, project)?;

    let state = SessionState {
        id: generate_session_id(),
        project_key: project_key.to_string(),
        project_display_name: project.display_name.clone(),
        tool: tool.to_string(),
        topic,
        start_time_utc: Utc::now(),
        cwd,
        transcript_path,
    };

    prepare_transcript(&state)?;
    store.save(&state)?;
    Ok(state)
}

pub fn end_session(store: &SessionStore) -> Result<SessionState> {
    let state = store
        .load()?
        .ok_or_else(|| anyhow!("no active session found"))?;
    store.clear()?;
    Ok(state)
}

pub fn session_status(store: &SessionStore) -> Result<Option<SessionState>> {
    store.load()
}

pub fn collect_git_info(project: &ProjectConfig) -> GitInfo {
    let Some(repo_root) = &project.repo_root else {
        return GitInfo {
            files_changed: "Repo root not configured.".to_string(),
            summary: "Repo root not configured.".to_string(),
        };
    };

    let status_summary = git_output(repo_root, &["status", "-sb"]).unwrap_or_else(|err| {
        format!("Git status unavailable: {err}")
    });

    let files_changed = git_output(repo_root, &["status", "--porcelain"]).unwrap_or_else(|err| {
        format!("Git status unavailable: {err}")
    });

    let files_changed = if files_changed.trim().is_empty() {
        "No changes detected.".to_string()
    } else {
        files_changed
    };

    GitInfo {
        files_changed,
        summary: status_summary,
    }
}

pub fn write_dev_log(
    config: &Config,
    project: &ProjectConfig,
    session: &SessionState,
    input: DevLogInput,
    git_info: GitInfo,
) -> Result<PathBuf> {
    let templates_root = resolve_in_vault(&config.vault_path, &config.templates_dir)?;
    let store = TemplateStore::new(templates_root);
    let template = store.load(&config.dev_log_template)?;

    let date = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut values = HashMap::new();
    values.insert("date", date);
    values.insert("project_display_name", session.project_display_name.clone());
    values.insert("tool", session.tool.clone());
    values.insert(
        "topic",
        session.topic.clone().unwrap_or_else(|| "N/A".to_string()),
    );
    values.insert("goal", input.goal);
    values.insert("files_changed", git_info.files_changed);
    values.insert("git_summary", git_info.summary);
    values.insert("summary", input.summary);
    values.insert("decision", input.decision);
    values.insert("rationale", input.rationale);
    values.insert("follow_up_tasks", input.follow_up_tasks);
    values.insert("transcript_path", session.transcript_path.display().to_string());
    values.insert(
        "transcript_link",
        format_obsidian_link(&config.vault_path, &session.transcript_path),
    );
    values.insert(
        "transcript_excerpt",
        read_transcript_excerpt(&session.transcript_path, 120, 8000),
    );

    let rendered = render_template(&template, &values);

    let log_root = resolve_in_vault(&config.vault_path, &project.dev_logs_dir)?;
    let filename = format!("dev-log-{}.md", Local::now().format("%Y%m%d-%H%M%S"));
    let path = log_root.join(filename);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create dev log directory {}", parent.display())
        })?;
    }
    fs::write(&path, rendered)
        .with_context(|| format!("Failed to write dev log {}", path.display()))?;
    Ok(path)
}

fn format_obsidian_link(vault_path: &Path, transcript_path: &Path) -> String {
    match transcript_path.strip_prefix(vault_path) {
        Ok(relative) => {
            let normalized = relative.to_string_lossy().replace('\\', "/");
            format!("[[{normalized}]]")
        }
        Err(_) => transcript_path.display().to_string(),
    }
}

fn read_transcript_excerpt(path: &Path, max_lines: usize, max_chars: usize) -> String {
    let raw = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) => return format!("Transcript unavailable: {err}"),
    };

    let mut lines: Vec<&str> = raw.lines().collect();
    if lines.len() > max_lines {
        lines = lines.split_off(lines.len() - max_lines);
    }
    let mut excerpt = lines.join("\n");
    if excerpt.len() > max_chars {
        let total_chars = excerpt.chars().count();
        let keep_chars = max_chars.min(total_chars);
        let skip_chars = total_chars.saturating_sub(keep_chars);
        excerpt = excerpt.chars().skip(skip_chars).collect();
        // Mark truncation so readers know this is not the full transcript.
        excerpt.insert_str(0, "...(truncated)\n");
    }
    if excerpt.trim().is_empty() {
        "Transcript is empty.".to_string()
    } else {
        excerpt
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PtyConfig {
    pub cols: u16,
    pub rows: u16,
}

pub fn run_tool_with_transcript(
    executable: &str,
    args: &[String],
    transcript_path: &Path,
    use_pty: bool,
    prefer_script: bool,
    pty: PtyConfig,
) -> Result<i32> {
    if use_pty {
        #[cfg(unix)]
        if prefer_script {
            match run_tool_with_transcript_script(executable, args, transcript_path) {
                Ok(code) => return Ok(code),
                Err(err) => eprintln!(
                    "[aiw] script backend unavailable, falling back to native PTY: {err}"
                ),
            }
        }
        return run_tool_with_transcript_pty(executable, args, transcript_path, pty);
    }
    run_tool_with_transcript_pipe(executable, args, transcript_path)
}

fn normalize_tool(tool: &str) -> Result<&'static str> {
    match tool.to_ascii_lowercase().as_str() {
        "claude" => Ok("claude"),
        "gemini" => Ok("gemini"),
        "codex" => Ok("codex"),
        _ => Err(anyhow!("unsupported tool: {tool}")),
    }
}

fn resolve_project<'a>(config: &'a Config, key: &str) -> Result<&'a ProjectConfig> {
    config
        .projects
        .get(key)
        .ok_or_else(|| anyhow!("project not found: {key}"))
}

fn build_transcript_path(config: &Config, project: &ProjectConfig) -> Result<PathBuf> {
    let root = resolve_in_vault(&config.vault_path, &project.transcript_dir)?;
    let date = Local::now().format("%Y-%m-%d").to_string();
    let filename = format!("session-{}.log", Local::now().format("%H%M%S"));
    Ok(root.join(date).join(filename))
}

fn generate_session_id() -> String {
    Local::now().format("%Y%m%d%H%M%S").to_string()
}

fn git_output(repo_root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .output()
        .with_context(|| "Failed to execute git")?;
    if !output.status.success() {
        return Err(anyhow!(
            "git command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn run_tool_with_transcript_pipe(
    executable: &str,
    args: &[String],
    transcript_path: &Path,
) -> Result<i32> {
    let mut child = Command::new(executable)
        .args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to start tool: {executable}"))?;

    let stdout = child.stdout.take().context("Failed to capture stdout")?;
    let stderr = child.stderr.take().context("Failed to capture stderr")?;

    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(transcript_path)
        .with_context(|| format!("Failed to open transcript {}", transcript_path.display()))?;
    let file = Arc::new(Mutex::new(file));

    let out_handle = spawn_tee(stdout, file.clone(), true);
    let err_handle = spawn_tee(stderr, file.clone(), false);

    let status = child
        .wait()
        .with_context(|| "Failed to wait for tool process")?;

    out_handle.join().ok();
    err_handle.join().ok();

    let code = status.code().unwrap_or(-1);
    let mut file = file.lock().expect("transcript file lock poisoned");
    writeln!(file, "\n\n[aiw] tool exited with code {code}\n").ok();

    Ok(code)
}

fn run_tool_with_transcript_pty(
    executable: &str,
    args: &[String],
    transcript_path: &Path,
    pty: PtyConfig,
) -> Result<i32> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: pty.rows,
            cols: pty.cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .with_context(|| "Failed to open PTY")?;

    let mut cmd = CommandBuilder::new(executable);
    for arg in args {
        cmd.arg(arg);
    }

    let mut child = pair
        .slave
        .spawn_command(cmd)
        .with_context(|| format!("Failed to start tool: {executable}"))?;
    drop(pair.slave);

    let reader = pair
        .master
        .try_clone_reader()
        .with_context(|| "Failed to clone PTY reader")?;
    let mut writer = pair
        .master
        .take_writer()
        .with_context(|| "Failed to open PTY writer")?;
    let _raw_mode_guard = TerminalRawModeGuard::new()?;

    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(transcript_path)
        .with_context(|| format!("Failed to open transcript {}", transcript_path.display()))?;
    let file = Arc::new(Mutex::new(file));

    let out_handle = spawn_pty_reader(reader, file.clone());
    let _in_handle = thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buffer = [0u8; 1024];
        loop {
            match stdin.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    if writer.write_all(&buffer[..n]).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let status = child
        .wait()
        .with_context(|| "Failed to wait for tool process")?;

    out_handle.join().ok();
    // Do not join the stdin forwarding thread here. It can block on stdin
    // reads even after the child exits, which would hang wrapper shutdown.

    let code = status.exit_code() as i32;
    let mut file = file.lock().expect("transcript file lock poisoned");
    writeln!(file, "\n\n[aiw] tool exited with code {code}\n").ok();

    Ok(code)
}

#[cfg(unix)]
fn run_tool_with_transcript_script(
    executable: &str,
    args: &[String],
    transcript_path: &Path,
) -> Result<i32> {
    let _raw_mode_guard = TerminalRawModeGuard::new()?;
    let command = build_script_command(executable, args);
    let status = Command::new("script")
        .arg("-q")
        .arg("-f")
        .arg("-e")
        .arg("-c")
        .arg(&command)
        .arg(transcript_path)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .with_context(|| "Failed to start script backend")?;

    let code = status
        .code()
        .or_else(|| status.signal().map(|signal| 128 + signal))
        .unwrap_or(-1);
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(transcript_path)
        .with_context(|| format!("Failed to open transcript {}", transcript_path.display()))?;
    writeln!(file, "\n\n[aiw] tool exited with code {code}\n").ok();
    Ok(code)
}

#[cfg(unix)]
fn build_script_command(executable: &str, args: &[String]) -> String {
    let mut parts = Vec::with_capacity(args.len() + 1);
    parts.push(shell_quote(executable));
    for arg in args {
        parts.push(shell_quote(arg));
    }
    parts.join(" ")
}

#[cfg(unix)]
fn shell_quote(input: &str) -> String {
    if input.is_empty() {
        return "''".to_string();
    }
    let escaped = input.replace('\'', "'\"'\"'");
    format!("'{escaped}'")
}

struct TerminalRawModeGuard {
    #[cfg(unix)]
    fd: i32,
    #[cfg(unix)]
    original: Option<libc::termios>,
}

impl TerminalRawModeGuard {
    fn new() -> Result<Self> {
        #[cfg(unix)]
        {
            let fd = std::io::stdin().as_raw_fd();
            let is_tty = unsafe { libc::isatty(fd) } == 1;
            if !is_tty {
                return Ok(Self { fd, original: None });
            }

            let mut termios = unsafe { std::mem::zeroed::<libc::termios>() };
            if unsafe { libc::tcgetattr(fd, &mut termios) } != 0 {
                return Err(anyhow!("Failed to read terminal attributes"));
            }
            let original = termios;
            unsafe { libc::cfmakeraw(&mut termios) };
            if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &termios) } != 0 {
                return Err(anyhow!("Failed to enable raw terminal mode"));
            }

            return Ok(Self {
                fd,
                original: Some(original),
            });
        }

        #[cfg(not(unix))]
        {
            Ok(Self {})
        }
    }
}

impl Drop for TerminalRawModeGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(original) = self.original.as_ref() {
            let _ = unsafe { libc::tcsetattr(self.fd, libc::TCSANOW, original) };
        }
    }
}

fn prepare_transcript(state: &SessionState) -> Result<()> {
    if let Some(parent) = state.transcript_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create transcript directory {}",
                parent.display()
            )
        })?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&state.transcript_path)
        .with_context(|| {
            format!(
                "Failed to open transcript {}",
                state.transcript_path.display()
            )
        })?;
    writeln!(
        file,
        "[aiw] session start: {}\n[aiw] project: {}\n[aiw] tool: {}\n[aiw] topic: {}\n[aiw] cwd: {}\n\n",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        state.project_display_name,
        state.tool,
        state.topic.clone().unwrap_or_else(|| "N/A".to_string()),
        state.cwd.display()
    )?;
    Ok(())
}

fn spawn_tee<R: Read + Send + 'static>(
    mut reader: R,
    file: Arc<Mutex<fs::File>>,
    to_stdout: bool,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = &buffer[..n];
                    let _ = if to_stdout {
                        let mut out = std::io::stdout();
                        let _ = out.write_all(chunk);
                        out.flush()
                    } else {
                        let mut out = std::io::stderr();
                        let _ = out.write_all(chunk);
                        out.flush()
                    };
                    if let Ok(mut file) = file.lock() {
                        let _ = file.write_all(chunk);
                    }
                }
                Err(_) => break,
            }
        }
    })
}

fn spawn_pty_reader<R: Read + Send + 'static>(
    mut reader: R,
    file: Arc<Mutex<fs::File>>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = &buffer[..n];
                    let mut out = std::io::stdout();
                    let _ = out.write_all(chunk);
                    let _ = out.flush();
                    if let Ok(mut file) = file.lock() {
                        let _ = file.write_all(chunk);
                    }
                }
                Err(_) => break,
            }
        }
    })
}

pub fn cleanup_transcript(path: &Path) -> Result<()> {
    let raw =
        fs::read(path).with_context(|| format!("Failed to read transcript {}", path.display()))?;
    let cleaned = strip_terminal_control_sequences(&raw);
    fs::write(path, cleaned)
        .with_context(|| format!("Failed to write transcript {}", path.display()))?;
    Ok(())
}

fn strip_terminal_control_sequences(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < input.len() {
        let b = input[i];
        if b == 0x1B {
            if i + 1 >= input.len() {
                break;
            }
            match input[i + 1] {
                b'[' => {
                    i += 2;
                    while i < input.len() {
                        let c = input[i];
                        if (0x40..=0x7E).contains(&c) {
                            i += 1;
                            break;
                        }
                        i += 1;
                    }
                    continue;
                }
                b']' => {
                    i += 2;
                    while i < input.len() {
                        if input[i] == 0x07 {
                            i += 1;
                            break;
                        }
                        if input[i] == 0x1B && i + 1 < input.len() && input[i + 1] == b'\\' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                    continue;
                }
                _ => {
                    i += 2;
                    continue;
                }
            }
        }

        if b == b'\r' {
            i += 1;
            continue;
        }
        if b == b'\n' || b == b'\t' || (0x20..=0x7E).contains(&b) {
            out.push(char::from(b));
        }
        i += 1;
    }
    out
}
