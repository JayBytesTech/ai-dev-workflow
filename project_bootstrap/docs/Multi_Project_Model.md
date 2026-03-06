# Multiple Project Support Model

Multiple projects are a first-class feature in v1.

## Why this matters
- one Obsidian vault may contain many projects
- transcript and decision memory should remain organized
- users may switch contexts frequently

## Project identity
Each project should have:
- internal key, e.g. `ai-hub`
- display name, e.g. `AI Hub`
- optional repo root path
- optional default note folders

## Example config
```toml
[projects.ai-hub]
display_name = "AI Hub"
repo_root = "/path/to/ai-hub"
dev_logs_dir = "Dev Logs/AI Hub"
adr_dir = "ADR/AI Hub"
transcript_dir = "AI Sessions/raw/AI Hub"
allowed_note_folders = ["Projects/AI Hub", "Research"]
```

## Effects of project selection
Selecting a project should determine:
- default repo root for git metadata
- where logs are written
- where ADRs are written
- which note paths are allowed
