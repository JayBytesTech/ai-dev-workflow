# Implementation Roadmap

This roadmap is deliberately staged so Codex or a human developer can produce working value early.

## Stage 0 - Repository scaffold
Goal: create the workspace, crates, and baseline docs.

Deliverables:
- Cargo workspace
- modular crate skeleton
- config sample
- templates
- docs copied into repo

## Stage 1 - Config and project model
Goal: make config the source of truth.

Implement:
- TOML config parsing
- project definitions
- path resolution
- executable mapping for claude/gemini/codex
- config validation command

Success check:
- `aiw config validate` works on a sample config

## Stage 2 - Session lifecycle
Goal: establish tracked sessions, even before perfect transcript capture.

Implement:
- session state file
- `session start`
- `session status`
- `session end`
- metadata persistence

Success check:
- session can start and end with project/tool/topic metadata

## Stage 3 - Dev log generation
Goal: write useful markdown output into the vault.

Implement:
- template loader
- dev log renderer
- project-aware dev log path generation
- git status and file change capture

Success check:
- ending a session creates a valid dev log markdown file

## Stage 4 - ADR flow
Goal: add optional decision promotion.

Implement:
- ADR prompt flow
- ADR renderer
- ADR path generation
- filename normalization

Success check:
- user can generate ADR from `session end` or `adr create`

## Stage 5 - Note scanning
Goal: detect inline AI commands safely.

Implement:
- markdown file loader
- command parser for:
  - `/ai summarize`
  - `/ai critique`
  - `/ai research`
  - `/ai extract-tasks`
- allowed-folder checks

Success check:
- `aiw note scan` reports commands correctly

## Stage 6 - AI tool adapters
Goal: invoke supported AI CLIs in a structured way.

Implement:
- adapter trait
- adapters for claude, gemini, codex
- explicit tool selection
- prompt construction helpers
- captured output handling

Success check:
- note processing can call a real adapter

## Stage 7 - Note processing
Goal: turn notes into AI-assisted workflows.

Implement:
- command execution
- append result blocks
- task extraction output formatting
- safe write-back

Success check:
- processing a note appends tool output to the note

## Stage 8 - Transcript capture improvement
Goal: refine transcript capture quality after core flows work.

Implement:
- transcript abstraction
- Linux and Windows handling
- better session capture mechanics
- improved storage layout

Success check:
- visible terminal activity is captured more reliably

## Stage 9 - Quality pass
Goal: make the tool shippable for early users.

Implement:
- tests
- path edge cases
- error handling polish
- docs cleanup
- install instructions
