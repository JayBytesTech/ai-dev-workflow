# System Architecture

## 1. Overview

`ai-dev-workflow` is a modular Rust CLI that sits between the developer's IDE workflow, supported AI CLIs, and an Obsidian vault.

```text
Developer
   |
   v
IDE / Terminal
   |
   v
AI CLI Tool  <->  aiw session wrapper / adapters
   |
   v
aiw core modules
   |
   +--> transcript storage
   +--> dev log generation
   +--> ADR generation
   +--> note command processing
   |
   v
Obsidian Vault
```

## 2. Architectural principles

- explicit over implicit
- modular over monolithic internals
- filesystem-first integration for v1
- project-aware paths
- adapter-based AI tool support
- minimal mutation of user notes
- cross-platform design for Linux and Windows

## 3. Major subsystems

### 3.1 CLI layer
Responsible for:
- command parsing
- user interaction
- prompts at session end
- dispatch into application services

Candidate crate:
- `crates/cli`

### 3.2 Config layer
Responsible for:
- parsing TOML configuration
- resolving paths
- validating project mappings
- exposing runtime settings

Candidate crate:
- `crates/config`

### 3.3 Session layer
Responsible for:
- session lifecycle
- metadata creation
- transcript capture orchestration
- session state persistence
- git metadata collection

Candidate crate:
- `crates/session`

### 3.4 AI tool adapter layer
Responsible for:
- per-tool executable mapping
- prompt invocation conventions
- output collection
- future tool extensibility

Candidate crate:
- `crates/ai_tools`

### 3.5 Vault / Obsidian layer
Responsible for:
- vault path handling
- project folder resolution
- log file output
- note reading / writing
- command scanning

Candidate crate:
- `crates/obsidian`

### 3.6 ADR layer
Responsible for:
- ADR prompt flow
- template rendering
- ADR filename generation

Candidate crate:
- `crates/adr`

### 3.7 Template layer
Responsible for:
- loading templates
- simple variable substitution
- default template fallback logic

Candidate crate:
- `crates/templates`

## 4. Data flow

### 4.1 Session logging flow

```text
aiw session start
  -> validate config
  -> resolve project
  -> resolve AI tool adapter
  -> create session metadata
  -> start transcript capture

developer uses AI CLI in tracked session

aiw session end
  -> stop transcript capture
  -> collect git metadata
  -> prompt for summary / decision / follow-ups
  -> write dev log
  -> offer ADR creation
  -> write ADR if confirmed
```

### 4.2 Note processing flow

```text
aiw note process --project ai-hub --tool gemini <path-to-note>
  -> read note
  -> detect inline /ai commands
  -> build prompt from note content + command intent
  -> invoke configured AI tool
  -> append result block to note
  -> save updated note
```

## 5. Storage model

All user-visible artifacts live inside the Obsidian vault.

Recommended structure:

```text
Vault/
├── Projects/
│   └── <Project Name>/
├── Dev Logs/
│   └── <Project Name>/
├── ADR/
│   └── <Project Name>/
├── AI Sessions/
│   └── raw/
│       └── <Project Name>/
├── Research/
└── Templates/
```

## 6. Session state

During an active tracked session, the tool should persist enough local state to resume `session end` safely.

Possible session state:
- active session id
- project name
- tool name
- start timestamp
- transcript target path
- working directory
- optional topic

The implementation should assume only one active local tracked session per config root in v1.

## 7. Cross-platform considerations

### Linux
- likely easier PTY and transcript handling
- filesystem path semantics straightforward

### Windows
- shell behavior and PTY handling differ
- path normalization and executable discovery require care

The transcript subsystem should be abstracted early to avoid baking in OS-specific assumptions.

## 8. Notes on transcript capture

The design target is to capture:
- user prompts
- model output
- visible terminal interaction

However, some AI CLIs may use advanced TUI behavior. The architecture should keep transcript capture behind an interface so implementations can be improved later without destabilizing the rest of the system.

## 9. Future extensibility

The architecture should support future additions without breaking the CLI surface:

- new AI adapters
- command router
- Obsidian plugin bridge
- task manager integrations
- session analytics
