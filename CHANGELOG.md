# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- Tag-based GitHub Release workflow with Linux, macOS, and Windows binaries.
- CI workflow enforcing formatting, clippy, and tests.
- Non-interactive `session end` mode with optional JSON output.
- `aiw search` command for searching vault files by keyword or phrase.
- `aiw completions <shell>` subcommand to generate shell completion scripts (bash, zsh, fish, etc.).
- Integration tests for `aiw search` and `aiw config init`.
- Unit tests for `aiw-templates` crate covering `render_template` and `TemplateStore`.

### Changed
- Transcript capture lifecycle now includes recoverable capture state and doctor repair flow.
- `aiw session start --wrap` now propagates the wrapped tool's exit code to the shell.
