# ai-dev-workflow

`ai-dev-workflow` is a Rust CLI that captures AI-assisted development sessions and stores transcripts, dev logs, and ADRs inside an Obsidian vault.

## Status
Stage 8 (partial): config parsing/validation, session start/end/status, dev log generation, ADR creation, note scan detection, AI tool adapters, note processing, and a transcript capture wrapper (pipe or PTY) are implemented.

## Build

```bash
cargo build
```

The binary is `aiw` and can be run from the workspace:

```bash
cargo run -p aiw -- --help
```

## Config

Create a starter config:

```bash
cargo run -p aiw -- config init
```

By default the CLI reads `ai-dev-workflow.toml` from the current directory. You can override with `--config <path>` or `AIW_CONFIG`.

Validate config:

```bash
cargo run -p aiw -- config validate
```

List projects:

```bash
cargo run -p aiw -- projects list
```

## CLI (planned)

```text
aiw config init
aiw config validate

aiw session start
aiw session end
aiw session status

aiw note process
aiw note scan

aiw adr create
aiw projects list
```

## Session Capture

Wrap a tool session and capture transcripts:

```bash
aiw session start --project ai-hub --tool claude --wrap
```

Optional: pass tool arguments and use a PTY for richer TUI capture:

```bash
aiw session start --project ai-hub --tool claude --wrap --pty --tool-args --model sonnet
```

## Templates

Default templates live in `templates/` and are copied from the bootstrap package:

- `templates/Dev_Log_Template.md`
- `templates/ADR_Template.md`

## Notes

- Raw transcripts and curated notes are stored separately.
- Explicit tool selection is required (claude, gemini, codex).
- Obsidian is treated as a filesystem vault in v1.
