# Module Specifications

## crates/cli
Responsibilities:
- command parsing
- interactive prompts
- dispatching to services

## crates/config
Responsibilities:
- config schema
- TOML parsing
- path normalization
- validation

## crates/session
Responsibilities:
- active session state
- metadata lifecycle
- transcript orchestration
- git collection

## crates/ai_tools
Responsibilities:
- adapter trait
- tool-specific invocations
- output capture

## crates/obsidian
Responsibilities:
- vault path resolution
- artifact output paths
- note reading and writing
- command parsing helpers

## crates/adr
Responsibilities:
- ADR data model
- creation flow
- filename generation

## crates/templates
Responsibilities:
- template loading
- rendering helpers
- placeholder substitution
