# CLI Specification

Binary: `aiw`

## Design goals
- one binary
- explicit subcommands
- predictable arguments
- minimal surprise

## Top-level commands

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

## 1. Config commands

### `aiw config init`
Create a starter configuration file.

Example:
```bash
aiw config init
```

### `aiw config validate`
Validate config paths, project entries, and configured executables.

Example:
```bash
aiw config validate
```

## 2. Session commands

### `aiw session start`
Start a tracked session.

Example:
```bash
aiw session start --project ai-hub --tool claude --topic "router architecture"
```

Arguments:
- `--project <name>` required
- `--tool <claude|gemini|codex>` required
- `--topic <text>` optional
- `--cwd <path>` optional, default current working directory

Behavior:
- validates config
- resolves project mapping
- resolves adapter
- creates active session state
- starts transcript capture guidance or wrapper mode

### `aiw session end`
End the active tracked session.

Example:
```bash
aiw session end
```

Behavior:
- resolves active session
- finalizes transcript
- collects git metadata
- prompts for:
  - summary
  - decision
  - rationale
  - follow-up tasks
- writes dev log
- asks whether to create ADR

### `aiw session status`
Show current tracked session state.

Example:
```bash
aiw session status
```

## 3. Note commands

### `aiw note scan`
Scan a note or folder for inline AI commands without executing them.

Example:
```bash
aiw note scan --project ai-hub --path "Research/router.md"
```

### `aiw note process`
Process a note containing inline AI commands.

Example:
```bash
aiw note process --project ai-hub --tool gemini --path "Research/router.md"
```

Arguments:
- `--project <name>` required
- `--tool <claude|gemini|codex>` required
- `--path <path>` required

Behavior:
- validates note path against allowed folders
- detects supported commands
- invokes selected tool
- appends results safely

## 4. ADR commands

### `aiw adr create`
Create an ADR directly without session flow.

Example:
```bash
aiw adr create --project ai-hub --title "rule-based routing for MVP"
```

Arguments:
- `--project <name>` required
- `--title <text>` required

Behavior:
- prompts for context, options, decision, consequences
- writes ADR file using template

## 5. Project commands

### `aiw projects list`
List configured projects.

Example:
```bash
aiw projects list
```

## Output style

The CLI should prefer:
- concise human-readable output
- clear file paths when artifacts are created
- explicit errors with next-step hints

Example success output:
```text
Created dev log:
<absolute path>

Created transcript:
<absolute path>
```
