---
name: mind-mesh-ask-skill
description: ACP-mode Ask — query MindMesh knowledge via mind-mesh CLI instead of native tools.
version: 1.1.0
---

# MindMesh Ask (ACP mode)

When MindMesh Ask runs in **ACP mode**, you do not have built-in function tools. Use the **`mind-mesh tools`** CLI to access the same knowledge layers.

## Storage model

Each project's knowledge lives at **`{repo}/.mind-mesh/`** (versioned with the repository). The desktop app keeps a local registry at `~/.mind-mesh/registry.json` — only repo pointers, not knowledge files.

## Environment variables (set by MindMesh)

| Variable | Purpose |
|----------|---------|
| `MIND_MESH_KNOWLEDGE_ROOT` | Current project's `.mind-mesh/` directory (absolute path) |
| `MIND_MESH_PROJECT_SLUG` | Current project slug |
| `MIND_MESH_REPO_PATH` | Repository root (for citations) |
| `MIND_MESH_ASK_SKILL` | Ask skill directory |

CLI without MindMesh UI: run inside a Git workspace, or pass `--repo-path` / set `MIND_MESH_REPO_PATH`.

## Three layers (same as native Ask)

| Layer | CLI | When |
|-------|-----|------|
| **Macro** | (preloaded in prompt) | Architecture overview — answer from prompt first |
| **Meso** | `mind-mesh tools read-context` | One section of `agent/context.md` |
| **Micro** | `grep-pack` → `read-pack-file` | Source code from repomix pack |

**Never** read the live repository filesystem. The repomix pack is authoritative for code.

## CLI reference

All commands output JSON to stdout.

```bash
# From repository root (auto-detects workspace), or:
# mind-mesh --repo-path /path/to/repo tools ...

# List indexed projects
mind-mesh tools list-projects

# Repomix pack metadata
mind-mesh tools pack-meta --project {slug}

# Grep agent/repomix.md
mind-mesh tools grep-pack --project {slug} --pattern "struct Foo"

# Read a file slice from the pack (≤150 lines recommended)
mind-mesh tools read-pack-file --project {slug} --file src/main.rs --start-line 1 --end-line 80

# Architecture context (optional section)
mind-mesh tools read-context --project {slug}
mind-mesh tools read-context --project {slug} --section "核心流程"

# Human / structured docs (paths relative to .mind-mesh/)
mind-mesh tools search --query "authentication" --project {slug}
mind-mesh tools read-doc --project {slug} --path human/1.概述.md
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
