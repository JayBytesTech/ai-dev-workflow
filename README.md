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

## Install

From a tagged release with Cargo:

```bash
cargo install --git https://github.com/JayBytesTech/ai-dev-workflow --tag v0.1.0 aiw
```

From GitHub Releases binaries:

1. Download the archive matching your OS/architecture from the Releases page.
2. Extract `aiw` (or `aiw.exe` on Windows).
3. Add the extracted binary to your `PATH`.

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

Show resolved config (after profile and environment overrides):

```bash
cargo run -p aiw -- --profile ci config show --resolved
```

List projects:

```bash
cargo run -p aiw -- projects list
```

## CLI (planned)

```text
aiw config init
aiw config validate
aiw config show [--resolved]

aiw session start
aiw session end
aiw session status
aiw session doctor --project ai-hub --tool codex [--repair]

aiw note process
aiw note scan

aiw adr create
aiw projects list
```

Use `--profile <name>` with any command to apply profile-specific overrides.

## Session Capture

Wrap a tool session and capture transcripts:

```bash
aiw session start --project ai-hub --tool claude --wrap
```

Optional: pass tool arguments and use a PTY for richer TUI capture:

```bash
aiw session start --project ai-hub --tool claude --wrap --pty --pty-cols 140 --pty-rows 40 --tool-args --model sonnet
```

Inspect and repair stale transcript capture state:

```bash
aiw session status
aiw session doctor --project ai-hub --tool codex
aiw session doctor --project ai-hub --tool codex --repair
```

Non-interactive session end (CI/script friendly):

```bash
aiw session end --non-interactive --output json \
  --goal "Ship release prep" \
  --summary "Completed smoke tests and docs updates" \
  --decision "Use staged rollout" \
  --rationale "Reduce rollout risk" \
  --follow-up-task "prepare release notes" \
  --no-adr
```

## Templates

Default templates live in `templates/` and are copied from the bootstrap package. The filenames are configurable via `dev_log_template` and `adr_template` in the TOML config:

- `templates/AIW_Dev_Log.md`
- `templates/AIW_ADR.md`

## Notes

- Raw transcripts and curated notes are stored separately.
- Explicit tool selection is required (claude, gemini, codex).
- Obsidian is treated as a filesystem vault in v1.

## Post-install Smoke Test

This sequence validates install + basic config wiring using local temp paths:

```bash
tmp_root="$(mktemp -d)"
mkdir -p "$tmp_root/vault/Templates" "$tmp_root/bin"
printf '#!/usr/bin/env sh\necho mock-tool\n' > "$tmp_root/bin/mocktool"
chmod +x "$tmp_root/bin/mocktool"

aiw config init --output "$tmp_root/ai-dev-workflow.toml"
sed -i "s|/path/to/ObsidianVault|$tmp_root/vault|g" "$tmp_root/ai-dev-workflow.toml"
sed -i "s|/path/to/projects/ai-hub|$tmp_root/repo|g" "$tmp_root/ai-dev-workflow.toml"
sed -i "s|claude-code|$tmp_root/bin/mocktool|g; s|gemini|$tmp_root/bin/mocktool|g; s|codex|$tmp_root/bin/mocktool|g" "$tmp_root/ai-dev-workflow.toml"

cp templates/AIW_Dev_Log.md "$tmp_root/vault/Templates/"
cp templates/AIW_ADR.md "$tmp_root/vault/Templates/"
mkdir -p "$tmp_root/repo"

aiw --config "$tmp_root/ai-dev-workflow.toml" config validate
aiw --config "$tmp_root/ai-dev-workflow.toml" session doctor --project ai-hub --tool codex
```

## Release Process

1. Update `crates/cli/Cargo.toml` version.
2. Update [`CHANGELOG.md`](CHANGELOG.md) under `## [Unreleased]`.
3. Commit and tag: `git tag vX.Y.Z && git push origin vX.Y.Z`.
4. GitHub Actions builds release archives and publishes a GitHub Release.
5. Edit release notes using [`.github/release-template.md`](.github/release-template.md).
