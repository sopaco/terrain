---
name: agent-architecture-skill
description: Generate architecture-level agent context (no code细节) for Terrain projects.
---

# Agent Architecture Context Skill

Produce `agent/context.md` — a **dense, architecture-oriented** document for Coding Agents and Ask mode.

## NOT this skill's job

- Full source code or compressed signatures (that's `agent/repomix.md` — Ask reads via grep/read_file)
- Human-friendly long-form narrative (that's `human/` Litho docs)

## Output contract

Write to `TERRAIN_AGENT_CONTEXT_OUTPUT` or the path given in the prompt.

Required sections (Markdown `##` headings), in the language specified by the
prompt's `LANGUAGE` directive (Chinese names below; English equivalents in
parentheses — the prompt's explicit section list takes precedence):

1. **项目概览** (Project Overview) — purpose, consumers, key constraints (≤120 words)
2. **架构设计** (Architecture) — containers, layers, major dependencies (table/bullets)
3. **模块地图** (Module Map) — table: Module | Responsibility | Primary paths (≤12 rows)
4. **核心流程** (Core Flows) — 2–4 critical flows as numbered steps (no code)
5. **技术选型** (Tech Stack) — stack, frameworks, infra (bullet list)
6. **系统边界** (System Boundaries) — external APIs, DBs, third-party, trust boundaries
7. **代码映射索引** (Code Map Index) — table: Concept | Location (paths only) | Notes (≤15 rows)

## Developer meta (`terrain-meta.json`)

Repositories may ship one or more **`terrain-meta.json`** files (repo root, `.terrain/`, or nested). Before generating context, Terrain **programmatically collects** referenced files and injects them into the LLM prompt. The bundle is also written to `agent/meta-inputs.md`.

See `terrain-meta.example.json` in this skill directory.

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

## ACP mode (Terrain)

When Terrain runs context generation in **ACP mode**, native function tools are unavailable. Use the **`terrain tools`** CLI instead:

```bash
terrain tools pack-meta --project {slug}
terrain tools grep-pack --project {slug} --pattern "module_name"
terrain tools read-pack-file --project {slug} --file src/foo.rs --start-line 1 --end-line 80
```

See `terrain-ask-skill` for the full CLI reference. Do not read the live repository filesystem.

Environment variables set by Terrain:

| Variable | Purpose |
|----------|---------|
| `TERRAIN_AGENT_ARCH_SKILL` | This skill directory |
| `TERRAIN_AGENT_CONTEXT_OUTPUT` | Write target for `agent/context.md` |
| `TERRAIN_KNOWLEDGE_ROOT` | Project `.terrain/` directory |
| `TERRAIN_PROJECT_SLUG` | Project slug |
| `TERRAIN_REPO_PATH` | Repository root |

## How Ask consumes this

- **Macro**: 项目概览 + 架构设计 + 模块地图 preloaded in the question
- **Meso**: other sections fetched on demand via `read_agent_context(section=…)`
- **Micro**: source code via `grep_agent_pack` → `read_agent_pack_file`
