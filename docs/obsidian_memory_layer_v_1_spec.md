# Obsidian Memory Layer v1 Specification

## Purpose

The Obsidian Memory Layer provides a structured, queryable knowledge system for the AI Hub. It allows AI providers (ChatGPT, Claude, Gemini, and later local models) to retrieve, reason over, and optionally write information into the user's Obsidian vault.

This system acts as the **persistent memory substrate** for the hub.

Goals:

- Enable Retrieval-Augmented Generation (RAG) over the vault
- Provide safe read/write access to markdown notes
- Maintain auditability and user approval for changes
- Respect workspace/project context
- Work locally without requiring cloud infrastructure

Non‑Goals (v1):

- Full semantic graph reasoning
- Autonomous write access to arbitrary vault locations
- Heavy background agents

---

# Architecture Overview

```
User
 │
Desktop App (Chat UI)
 │
AI Hub
 │
├ Provider Router
├ Tool Runtime
├ Conversation Manager
└ Memory Layer
       │
       ├ Vault Connector
       ├ Indexer
       ├ Retriever
       ├ Context Pack Builder
       └ Write Manager

Obsidian Vault
```

The Memory Layer exposes tools used by the hub and agents:

- search_vault
- read_note
- write_note
- append_note
- list_notes

---

# Vault Connector

The Vault Connector is responsible for safe filesystem access.

Responsibilities:

- Discover vault root
- Enumerate markdown files
- Read file contents
- Write files within approved paths
- Watch filesystem changes

Implementation approach:

- Node.js filesystem watcher (chokidar recommended)
- Ignore hidden/system folders

Ignored folders:

```
.obsidian/
.trash/
node_modules/
```

---

# Folder Conventions

The system assumes the following structure inside the vault:

```
AI Memory/
Daily/
Weekly/
Monthly/
Projects/
Work Hub/
Study Hub/
Knowledge/
Templates/
AI Conversations/
AI Drafts/
```

Write operations are restricted to:

```
AI Drafts/
AI Conversations/
```

unless explicitly approved.

---

# Context Files

The AI Hub determines user context from three files.

```
AI Memory/Active Workspace.md
AI Memory/Active Project.md
AI Memory/Active Study Track.md
```

Each file contains a single value.

Example:

```
Development
AI Hub
AWS Certification
```

These values influence retrieval weighting.

---

# Indexing Strategy

v1 uses **hybrid search**.

Phase 1:

- keyword index
- markdown parsing

Phase 2 (optional upgrade):

- vector embeddings


## Chunking

Notes are split into chunks.

Chunk size:

```
400–1000 tokens
```

Metadata stored per chunk:

```
path
note_title
tags
last_modified
workspace_hint
project_hint
```

---

# Retrieval Pipeline

1. User prompt received
2. Active context read
3. Query executed against index
4. Candidate chunks retrieved
5. Top K chunks selected
6. Context pack assembled

Typical values:

```
Initial candidates: 30
Final context: 6–10
```


## Context Weighting

Files receive score boosts when:

Workspace matches
Project matches
Recent modification

Example boost rules:

```
+2.0 Active project folder
+1.5 Workspace folder
+1.0 Recent notes (<7 days)
```

---

# Context Pack Builder

The context pack is what the LLM actually sees.

Structure:

```
SYSTEM CONTEXT

Workspace: Development
Project: AI Hub
Study Track: AWS Certification

---

Relevant Notes:

[1] Projects/AI Hub/Architecture.md
<content snippet>

[2] Daily/2026-03-04.md
<content snippet>

[3] Knowledge/RAG Basics.md
<content snippet>

---

User Question
```

The model should cite the note paths in responses.

---

# Tool API

## search_vault

Search notes by keyword or semantic similarity.

Input:

```
query
limit
workspace_context
```

Output:

```
list of chunk results
```

---

## read_note

Read full markdown note.

Input:

```
path
```

Output:

```
markdown content
```

---

## list_notes

List notes under a folder.

Input:

```
folder
limit
```

---

## write_note

Create or replace note content.

Restrictions:

```
Allowed folders:
AI Drafts/
AI Conversations/
```

Requires approval.

---

## append_note

Append content to existing note.

Use cases:

- conversation logs
- AI suggestions

---

# Write Approval System

All write operations follow this flow:

1. AI proposes change
2. UI shows preview
3. User approves
4. Write occurs
5. Action logged

Preview example:

```
Proposed write:
Projects/AI Hub/PRD Draft.md

+ new section
+ architecture diagram
```

---

# Audit Log

Every tool call is logged.

Storage location:

```
AI Memory/Audit Log.md
```

Example entry:

```
2026-03-05
Tool: write_note
Target: AI Drafts/architecture-summary.md
Approved: Yes
```

---

# Performance Goals

Search latency target:

```
< 200 ms
```

Context assembly target:

```
< 50 ms
```

Indexing should run:

- on vault startup
- on file changes

---

# Future Enhancements

## Vector Search

Add embedding support using:

- OpenAI
- local embedding models

Recommended DB options:

```
LanceDB
Chroma
Qdrant
```


## Knowledge Graph

Extract structured relationships:

```
concept → note
project → decisions
study topic → related notes
```


## Agents

Possible agents:

- Daily summarizer
- Study coach
- Project planner
- Knowledge curator

---

# Minimal v1 Implementation Stack

Recommended technologies:

Hub Layer

```
TypeScript
Node.js
```

Search

```
MiniSearch (keyword)
```

Filesystem

```
chokidar
```

Vector DB (optional later)

```
LanceDB
```

---

# Development Milestones

## Phase 1

- Vault connector
- Basic search
- read_note tool

## Phase 2

- context pack builder
- workspace/project weighting

## Phase 3

- write approval system
- audit log

## Phase 4

- vector search
- agents

---

# Summary

The Obsidian Memory Layer turns the user's vault into a persistent knowledge base that AI models can retrieve from safely.

It enables:

- personal knowledge retrieval
- project awareness
- study assistance
- conversation memory

while preserving user control over all modifications.

