---
name: agent-architecture-skill
description: Generate architecture-level agent context (no code细节) for MindMesh projects.
---

# Agent Architecture Context Skill

Produce `agent/context.md` — a **dense, architecture-oriented** document for Coding Agents and Ask mode.

## NOT this skill's job

- Full source code or compressed signatures (that's `agent/repomix.md` — Ask reads via grep/read_file)
- Human-friendly long-form narrative (that's `human/` Litho docs)

## Output contract

Write to `MIND_MESH_AGENT_CONTEXT_OUTPUT` or the path given in the prompt.

Required sections (Markdown `##` headings):

1. **项目概览** — purpose, consumers, key constraints (≤120 words)
2. **架构设计** — containers, layers, major dependencies (table/bullets)
3. **模块地图** — table: Module | Responsibility | Primary paths (≤12 rows)
4. **核心流程** — 2–4 critical flows as numbered steps (no code)
5. **技术选型** — stack, frameworks, infra (bullet list)
6. **系统边界** — external APIs, DBs, third-party, trust boundaries
7. **代码映射索引** — table: Concept | Location (paths only) | Notes (≤15 rows)

## Developer meta (`mind-mesh-meta.json`)

Repositories may ship one or more **`mind-mesh-meta.json`** files (repo root, `.mind-mesh/`, or nested). Before generating context, MindMesh **programmatically collects** referenced files and injects them into the LLM prompt. The bundle is also written to `agent/meta-inputs.md`.

See `mind-mesh-meta.example.json` in this skill directory.

### Schema (version 1)

```json
{
  "version": 1,
  "hints": {
    "module_roots": ["crates/", "src/"],
    "notes": "Free-form team hints"
  },
  "inputs": [
    { "label": "Modules doc", "type": "file", "path": "docs/architecture.md" },
    { "label": "ADRs", "type": "glob", "pattern": "docs/adr/*.md", "optional": true },
    { "label": "Glossary", "type": "inline", "content": "..." }
  ]
}
```

Input types: `file` (path relative to repo or meta file), `glob`, `inline`. Each input may set `optional` and `max_chars` (default 3500).

### How to use meta in output

- Treat collected meta as **authoritative** for **模块地图** and **系统边界**
- Supplement with `grep_agent_pack` for path discovery only
- Do **not** invent modules that contradict developer meta
- Rule-based `modules/` stubs are **deprecated** — synthesis is LLM-driven from meta + repomix

## Rules

- **Hard limit: ≤14000 characters** — trimmed to 16 KiB on save if exceeded
- Macro sections (1–3) must be the densest; sections 4–7 can be shorter
- No function bodies, no code blocks > 3 lines
- Path references only — implementation detail lives in `agent/repomix.md`
- Use `grep_agent_pack` only to discover paths, never paste grep output
- Prefer tables and bullet lists over prose

## ACP mode (MindMesh)

When MindMesh runs context generation in **ACP mode**, native function tools are unavailable. Use the **`mind-mesh tools`** CLI instead:

```bash
mind-mesh tools pack-meta --project {slug}
mind-mesh tools grep-pack --project {slug} --pattern "module_name"
mind-mesh tools read-pack-file --project {slug} --file src/foo.rs --start-line 1 --end-line 80
```

See `mind-mesh-ask-skill` for the full CLI reference. Do not read the live repository filesystem.

Environment variables set by MindMesh:

| Variable | Purpose |
|----------|---------|
| `MIND_MESH_AGENT_ARCH_SKILL` | This skill directory |
| `MIND_MESH_AGENT_CONTEXT_OUTPUT` | Write target for `agent/context.md` |
| `MIND_MESH_KNOWLEDGE_ROOT` | Project `.mind-mesh/` directory |
| `MIND_MESH_PROJECT_SLUG` | Project slug |
| `MIND_MESH_REPO_PATH` | Repository root |

## How Ask consumes this

- **Macro**: 项目概览 + 架构设计 + 模块地图 preloaded in the question
- **Meso**: other sections fetched on demand via `read_agent_context(section=…)`
- **Micro**: source code via `grep_agent_pack` → `read_agent_pack_file`
