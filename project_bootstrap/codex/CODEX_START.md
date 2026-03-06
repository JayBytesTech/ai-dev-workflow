You are implementing a Rust project called `ai-dev-workflow`.

Read every document in this bootstrap package before writing code.

Priority order:
1. docs/PRD.md
2. docs/Architecture.md
3. docs/CLI_Spec.md
4. docs/Implementation_Roadmap.md
5. docs/Module_Specs.md
6. docs/Multi_Project_Model.md
7. templates/
8. config/ai-dev-workflow.example.toml
9. repo_structure/REPO_STRUCTURE.md

Implementation instructions:

- Build this as a Rust workspace.
- Keep the binary name as `aiw`.
- Support multiple projects from day one.
- Support Linux and Windows first.
- Start with explicit tool selection for Claude Code, Gemini CLI, and OpenAI Codex CLI.
- Treat Obsidian as a filesystem vault in v1.
- Keep calendar integration out of scope.
- Do not build routing logic in v1.
- Prefer working, testable increments over ambitious abstractions.

Execution strategy:

1. Scaffold the workspace and crates.
2. Implement config parsing and validation first.
3. Implement session state and start/end/status commands.
4. Implement dev log generation.
5. Implement ADR creation flow.
6. Implement note scanning.
7. Implement AI tool adapters.
8. Implement note processing.
9. Improve transcript capture.

Important constraints:

- Be explicit rather than magical.
- Avoid silently mutating unrelated note content.
- Keep raw transcripts and curated notes in separate paths.
- Assume transcript capture quality may need iteration.
- Make code readable and maintainable enough for open-source release.

When you start coding, also create:
- repo README
- sample config
- sample templates
- basic install/build instructions
