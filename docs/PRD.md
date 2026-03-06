# Product Requirements Document
## Project: ai-dev-workflow

## 1. Purpose

`ai-dev-workflow` is a Rust-based CLI tool for AI-assisted software development workflows. It is designed as an installable open-source tool, with architecture clean enough to evolve into a product later.

The system connects four working layers:

1. IDE-based development
2. AI CLI tools
3. structured session logging
4. an Obsidian vault as long-term searchable memory

The project must help developers preserve the reasoning behind technical work, not just the code changes.

## 2. Product Vision

Developers increasingly use CLI-based AI tools during coding, architecture design, research, and debugging. Those interactions often contain important decisions and rationale that are lost once the terminal session ends.

`ai-dev-workflow` solves that by making AI-assisted development traceable and recoverable.

The tool will:

- log AI-assisted sessions
- preserve raw transcripts
- summarize decisions
- optionally promote important outcomes into ADRs
- allow markdown notes in Obsidian to trigger AI CLI actions
- keep development and memory clearly separated

Core philosophy:

```text
IDE = Build
AI CLI = Collaborate
Obsidian = Remember
ADR = Explain why
```

## 3. Target Users

### Primary user
A developer who works in an IDE, uses CLI-based AI tools, and keeps long-term notes in Obsidian.

### Secondary user
An open-source user who wants installable tooling for AI-assisted development memory and traceability.

### Future user
Teams or product users who may later want plugin systems, richer routing, analytics, or UI layers.

## 4. Goals

### Primary goals
- Preserve AI-assisted development context
- Capture raw transcripts and readable summaries
- Connect code work to long-term knowledge notes
- Maintain low-friction workflow
- Support multiple projects from the start
- Use AI subscriptions through official CLIs rather than building around API cost first

### Secondary goals
- Provide a clean Rust architecture
- Make the tool installable and extensible
- Keep Obsidian integration plugin-free in v1
- Enable future productization

## 5. Non-Goals for v1

The following are explicitly out of scope for v1:

- Obsidian plugin development
- autonomous multi-agent systems
- automatic model routing
- calendar integration
- vault-wide semantic indexing
- cloud sync features
- web UI
- team collaboration workflows
- automatic ADR generation without user confirmation

## 6. Platform Scope

### v1 supported environments
- Linux
- Windows

### future
- macOS

The design must avoid making Linux-only assumptions.

## 7. Required Integrations

### Day-one AI CLI integrations
- Claude Code
- Gemini CLI
- OpenAI Codex CLI

The product should treat AI tool invocation as adapter-based so more tools can be added later.

## 8. Core Functional Requirements

### FR-1. Session management
The tool must support manually started tracked sessions.

Example:

```bash
aiw session start --project ai-hub --tool claude --topic "router architecture"
aiw session end
```

The start command must:
- initialize session metadata
- identify project
- identify selected AI tool
- record topic if supplied
- begin transcript capture

The end command must:
- close transcript capture
- collect git status and file changes when available
- prompt for summary information
- generate a development log
- optionally offer ADR creation

### FR-2. Transcript capture
The system must capture a raw terminal transcript for tracked sessions.

Transcript capture target:
- user prompts entered into the AI CLI
- tool output / model responses visible in terminal
- other terminal interaction that is part of the session

The transcript should be stored under a project-aware raw path inside the Obsidian vault.

Example:

```text
AI Sessions/raw/AI Hub/2026-03-05/...
```

### FR-3. Development log generation
Each completed session must create a markdown development log in the vault.

The log must include:
- timestamp
- project
- AI tool used
- topic
- files changed
- git status summary
- user-entered summary
- final decision
- rationale
- follow-up tasks
- link/path to transcript

### FR-4. ADR support
At session end, the system must ask whether the session should create an ADR.

If the user says yes, the tool must create an ADR markdown file using the configured template.

ADR creation should be suggested, not forced.

### FR-5. Obsidian note command processing
The tool must scan markdown notes for inline AI commands.

Initial supported commands:
- `/ai summarize`
- `/ai critique`
- `/ai research`
- `/ai extract-tasks`

The system must:
1. read the note
2. detect valid inline command(s)
3. invoke the selected AI CLI tool explicitly
4. append results back into the note

### FR-6. Task extraction
The `/ai extract-tasks` command must write markdown task items into the note.

Output example:

```markdown
- [ ] implement router scoring
- [ ] add telemetry
- [ ] benchmark claude vs gemini
```

### FR-7. Multiple project support
The tool must support multiple projects from day one.

Project identity should affect:
- transcript paths
- dev log paths
- ADR paths
- default note processing scope
- metadata

### FR-8. Configuration
The tool must use a single TOML configuration file in v1.

Configuration must support:
- vault path
- project mappings
- default folders
- allowed note-processing folders
- AI tool executable mappings
- template paths
- transcript storage paths

### FR-9. Modular CLI
The product must expose a single CLI binary with subcommands.

Binary name:
```text
aiw
```

## 9. Usability Requirements

- Commands should be understandable without memorizing hidden behavior
- v1 should prefer explicitness over magic
- AI tool selection should be explicit, not routed automatically
- note processing should be limited to configured folders
- workflows should not require an Obsidian plugin

## 10. Security and Safety Requirements

- the system must not process arbitrary vault folders unless configured
- AI note command processing should only run in allowed folders
- command execution must use known configured tool adapters
- logs should clearly separate raw transcripts from curated knowledge docs
- the tool should never silently overwrite unrelated note content

## 11. Success Criteria

v1 is successful if a user can:

1. start and end a tracked AI-assisted dev session
2. produce a raw transcript and a readable dev log
3. optionally create an ADR
4. process a markdown note with inline AI commands
5. extract tasks into markdown
6. use the tool across multiple projects
7. keep all resulting artifacts organized in one Obsidian vault

## 12. Future Extensions

Potential v2+ directions:
- plugin-based Obsidian integration
- model routing
- richer note commands
- calendar/task manager integrations
- analytics across sessions
- semantic project memory
- local model support
