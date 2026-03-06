# Suggested Repository Structure

```text
ai-dev-workflow/
├── Cargo.toml
├── Cargo.lock
├── LICENSE-MIT
├── LICENSE-APACHE
├── README.md
├── docs/
├── templates/
├── config/
├── crates/
│   ├── cli/
│   ├── config/
│   ├── session/
│   ├── ai_tools/
│   ├── obsidian/
│   ├── adr/
│   └── templates/
├── tests/
└── examples/
```

## Notes
- keep docs checked into repo from day one
- use workspace crates even if some are small initially
- avoid premature micro-crates beyond the modules already identified
