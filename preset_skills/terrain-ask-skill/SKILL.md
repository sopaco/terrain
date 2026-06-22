---
name: terrain-ask-skill
description: Terrain Ask — query knowledge via terrain CLI when Agent execution mode is ACP.
version: 1.1.0
---

# Terrain Ask (ACP mode)

When Terrain runs in **ACP Agent execution mode**, built-in function tools are unavailable for Ask. Use the **`terrain tools`** CLI to access the same knowledge layers.

## Storage model

Each project's knowledge lives at **`{repo}/.terrain/`** (versioned with the repository). The desktop app keeps a local registry at `~/.terrain/registry.json` — only repo pointers, not knowledge files.

## Environment variables (set by Terrain)

| Variable | Purpose |
|----------|---------|
| `TERRAIN_KNOWLEDGE_ROOT` | Current project's `.terrain/` directory (absolute path) |
| `TERRAIN_PROJECT_SLUG` | Current project slug |
| `TERRAIN_REPO_PATH` | Repository root (for citations) |
| `TERRAIN_ASK_SKILL` | Ask skill directory |

CLI without Terrain UI: run inside a Git workspace, or pass `--repo-path` / set `TERRAIN_REPO_PATH`.

## Three layers (same as native Ask)

| Layer | CLI | When |
|-------|-----|------|
| **Macro** | (preloaded in prompt) | Architecture overview — answer from prompt first |
| **Meso** | `terrain tools read-context` | One section of `agent/context.md` |
| **Micro** | `grep-pack` → `read-pack-file` | Source code from repomix pack |

**Never** read the live repository filesystem. The repomix pack is authoritative for code.

## CLI reference

All commands output JSON to stdout.

```bash
# From repository root (auto-detects workspace), or:
# terrain --repo-path /path/to/repo tools ...

# List indexed projects
terrain tools list-projects

# Repomix pack metadata
terrain tools pack-meta --project {slug}

# Grep agent/repomix.md
terrain tools grep-pack --project {slug} --pattern "struct Foo"

# Read a file slice from the pack (≤150 lines recommended)
terrain tools read-pack-file --project {slug} --file src/main.rs --start-line 1 --end-line 80

# Architecture context (optional section)
terrain tools read-context --project {slug}
terrain tools read-context --project {slug} --section "核心流程"

# Human / structured docs (paths relative to .terrain/)
terrain tools search --query "authentication" --project {slug}
terrain tools read-doc --project {slug} --path human/1.概述.md
```

## Workflow

1. Use macro context from the user message when sufficient.
2. For architecture sections not preloaded → `read-context --section "…"`.
3. For implementation → `grep-pack` with a focused pattern, then `read-pack-file` with line range.
4. Cite paths as `src/foo.rs:42` from pack line numbers.
5. Do not run identical CLI commands twice with the same arguments.

## Limits

- `read-pack-file`: pass `--start-line` and `--end-line`; max ~150 lines per call.
- `search`: limit 5–10 hits for Ask.
- Do not dump entire `repomix.md`.
