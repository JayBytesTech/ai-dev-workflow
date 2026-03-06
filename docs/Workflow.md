# Intended User Workflow

## 1. Development workflow
The developer works in VS Code or another IDE and uses an AI CLI in the terminal.

They choose to track a meaningful session.

Example:
```bash
aiw session start --project ai-hub --tool claude --topic "router architecture"
```

They then work normally.

At the end:
```bash
aiw session end
```

The tool generates:
- raw transcript path
- dev log markdown
- optional ADR

## 2. Knowledge workflow
The developer keeps normal long-term notes in Obsidian.

Examples:
- project notes
- research notes
- architecture notes
- personal technical notes

Those notes remain normal markdown files.

## 3. Obsidian AI workflow
A note can include commands like:

```text
/ai summarize
/ai critique
/ai research
/ai extract-tasks
```

The developer runs:

```bash
aiw note process --project ai-hub --tool gemini --path "Research/router.md"
```

The note is updated with appended AI output.

## 4. Decision memory workflow
When a session reaches an architectural conclusion, the user can promote it to an ADR.

That preserves:
- context
- options
- decision
- consequences

This avoids burying key reasoning inside raw transcripts.
