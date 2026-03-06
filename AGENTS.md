# Repository Guidelines

## Project Structure & Module Organization
`ai-dev-workflow` is a Rust workspace. Main code lives in `crates/`:
- `crates/cli`: `aiw` command entrypoint and subcommands.
- `crates/config`: TOML parsing, project model, validation.
- `crates/session`: session lifecycle and transcript capture.
- `crates/ai_tools`: adapters for `claude`, `gemini`, `codex`.
- `crates/obsidian`, `crates/adr`, `crates/templates`: note processing and markdown rendering.

Reference assets:
- `templates/`: default markdown templates.
- `config/`: sample config files.
- `docs/`: PRD, roadmap, and smoke-test docs.
- `project_bootstrap/`: starter package mirror for new projects.

## Build, Test, and Development Commands
- `cargo build`: build the full workspace.
- `cargo run -p aiw -- --help`: run the CLI locally.
- `cargo run -p aiw -- config validate`: validate `ai-dev-workflow.toml`.
- `cargo test`: run all unit/integration tests across crates.
- `cargo check`: fast compile check before commits.
- `cargo fmt`: apply standard Rust formatting.

Run commands from repo root: `/home/jaybytestech/src/portfolio/ai-dev-workflow`.

## Coding Style & Naming Conventions
Use Rust 2021 defaults and keep behavior explicit. Prefer small, focused functions and minimal filesystem mutation.
- Formatting: always run `cargo fmt`.
- Naming: `snake_case` for functions/modules/files, `PascalCase` for types, `UPPER_SNAKE_CASE` for constants.
- Errors: return `anyhow::Result` in command paths; avoid panics in non-test code.

## Testing Guidelines
Tests are currently crate-local, mainly in `crates/*/tests` and inline module tests.
- Add tests for new parsing, path resolution, and file-write behavior.
- Name tests by behavior (example: `validates_missing_tool_path`).
- Run `cargo test` before opening a PR; include regression tests for bug fixes.

## Commit & Pull Request Guidelines
Recent history uses short, imperative commit subjects (example: `Add session doctor diagnostics command`, `Fix Codex PTY passthrough`).
- Keep subject lines concise and action-first (`Add`, `Fix`, `Refactor`, `Test`, `Docs`).
- PRs should include: problem statement, scope, test evidence (`cargo test`/manual smoke checks), and docs/config updates when behavior changes.
- Keep changes focused; avoid unrelated refactors in the same PR.
