# ai-dev-workflow Bootstrap Package

Created: 2026-03-06

This package is the from-scratch bootstrap for `ai-dev-workflow`, a Rust-based workflow tool that connects:

- IDE-based development
- AI CLI tools
- structured session logging
- an Obsidian vault as a searchable second brain

## v1 scope

- Multi-project support
- Linux and Windows first
- Rust modular CLI
- Claude Code, Gemini CLI, and OpenAI Codex CLI
- Session logging with transcript capture
- Dev log generation into Obsidian
- Optional ADR creation prompt
- Auto-generation for dev logs and ADRs (`--auto`, `--auto-adr`)
- Inline Obsidian AI note commands
- Task extraction into markdown

## Recommended next move

1. Open this folder in VS Code.
2. Start a Codex session in the project root.
3. Paste `codex/CODEX_START.md` into the session.
4. Have Codex read the docs in `docs/` and work through `docs/Implementation_Roadmap.md`.

## Package layout

```text
project_bootstrap/
├── README.md
├── docs/
├── templates/
├── config/
├── repo_structure/
└── codex/
```
