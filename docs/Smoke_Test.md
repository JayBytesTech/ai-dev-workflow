# Smoke Test Checklist

This is a quick end-to-end sanity check for `ai-dev-workflow`.

## 1) Config sanity

```bash
aiw config validate
aiw projects list
```

## 2) Start a session (no wrap)

```bash
aiw session start --project ai-hub --tool claude --topic "smoke test"
aiw session status
```

## 3) End session + dev log

```bash
aiw session end
```

- Provide short values at the prompts.
- Verify a dev log appears in your vault at `Dev Logs/AI Hub/`.

## 4) ADR flow

```bash
aiw adr create --project ai-hub --title "smoke test ADR"
```

- Fill prompts and confirm ADR file exists in `ADR/AI Hub/`.

## 5) Note scan

Create a note under an allowed folder, for example:

```
/home/jaybytestech/Documents/Vault Redux/Compendium Redux/Projects/AI Hub/smoke.md
```

Content:

```
/ai summarize
/ai extract-tasks
```

Then run:

```bash
aiw note scan --project ai-hub --path "Projects/AI Hub/smoke.md"
```

## 6) Note process (tool required)

```bash
aiw note process --project ai-hub --tool gemini --path "Projects/AI Hub/smoke.md"
```

- Verify `## AIW Results` blocks are appended.
- Re-run to confirm it does not duplicate (idempotent).

## 7) Transcript capture (optional)

```bash
aiw session start --project ai-hub --tool claude --wrap --pty --tool-args --model sonnet
```

- Use the tool briefly, then exit.
- Confirm a transcript file exists in `AI Sessions/raw/AI Hub/<date>/`.

## 8) Capture health + repair

```bash
aiw session doctor --project ai-hub --tool claude
```

- If the tool or shell crashed during a wrapped session and doctor reports stale `capturing` state, run:

```bash
aiw session doctor --project ai-hub --tool claude --repair
```
