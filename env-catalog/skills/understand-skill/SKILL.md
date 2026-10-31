---
name: understand-skill
description: Use when a coding agent must produce or update the semantic knowledge graph (LLM-assisted) of this repo, run the Understand-Anything pipeline, or answer semantic-layer questions (domains, personas, tours, versions). Clean-room wrapper — UA's own skill prose is not copied.
version: 1.0.0
---

# Understand Skill (Understand-Anything wrapper)

[Understand-Anything](https://github.com/Egonex-AI/Understand-Anything) (MIT)
turns code into a **semantic knowledge graph** — LLM semantics (domains, personas,
tours, diff between versions) layered over tree-sitter structure. Terrain's graphify
covers the AST-only layer at zero cost; this skill adds the LLM-semantic layer,
opt-in and cost-flagged.

## Read-first rule

- Semantic/domain/persona questions → read `.terrain/understand/index.md` first.
- Architecture/symbol questions → `.terrain/graphify-out/GRAPH_REPORT.md` first
  (AST layer, zero cost); the understand graph is the semantic layer on top.
- Raw JSON graphs are not for direct reading — use the viewer or the skill flow.

## Data dirs (Terrain convention)

- **Working dir: `.ua/`** — where UA's pipeline writes; its resolver is fixed
  (the read-only viewer serves `.ua/` + legacy dir only).
- **Memory bank: `.terrain/understand/`** — agent-written `index.md` + summary
  markdown read by the memory bank (knowledge search, citations, agent context);
  raw JSON stays in `.ua/` (regenerable machine output — not archived).

## Toolchain check (before running)

1. Node ≥ 18: `node --version`
2. Viewer dist: `test -d ~/.terrain/understand-anything/repo/understand-anything-plugin/packages/viewer/dist`
3. Plugin: `test -L ~/.understand-anything-plugin || test -d ~/.claude/plugins/understand-anything`

## Run the pipeline (assistant-driven)

When the plugin is present, run UA's `/understand [path] [--full] [--auto-update]`
flow — its own SKILL.md governs the 7 phases, subagent orchestration, LLM usage
and its cost warnings (>100 files needs confirmation; re-scan ≈157k tokens
without preserved scan-result.json). Do NOT re-implement the pipeline. After it
finishes:

1. Write `.terrain/understand/index.md` — links to `.ua/knowledge-graph.json`,
   `meta.json` facts (gitCommitHash, lastAnalyzedAt), layers list, cross-links
   `../graphify-out/GRAPH_REPORT.md` (AST layer) and `../agent/context.md`.
2. Optionally more summary markdown for domains/personas worth keeping.
3. Raw artifacts stay in `.ua/` (regenerable machine output; not archived).

## Incremental update flow

- UA's plugin hook reminds the agent on git commit/merge/rebase (autoUpdate flag
  in .ua/config.json). Without the plugin: compare `.ua/meta.json` gitCommitHash
  vs `git rev-parse HEAD` — mismatch → run /understand --auto-update.
- AST-only layer stays on graphify: `GRAPHIFY_OUT=.terrain/graphify-out graphify update .`
  (zero cost, no LLM).

## Viewer (read-only, no LLM)

```bash
# npm bin (if published) or the built checkout:
node ~/.terrain/understand-anything/repo/understand-anything-plugin/packages/viewer/bin/viewer.mjs <project-dir> --no-open
```

Serves `.ua/` (or legacy dir) at 127.0.0.1 with a one-time `?token=`.
Raw JSON is regenerable machine output (gitignored).