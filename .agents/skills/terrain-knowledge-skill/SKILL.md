---
name: terrain-knowledge-skill
description: Use when a coding agent needs project knowledge from Terrain .terrain/ assets. Guides layered reading of context, private knowledge, and repomix index.
version: 1.1.0
---

# Terrain Knowledge Skill

Terrain stores **AI knowledge assets** under **`.terrain/`** in this repository (not a global `~/.terrain/knowledge` directory). The desktop app registry at `~/.terrain/registry.json` only maps slugs → repo paths.

Load **`rtk-skill`** when you need to run shell commands during investigation (git, grep repomix file, tests). Resolve tools via **conventional paths** (`~/.terrain/bin/…`) with `bunx`/`npx` fallback — see `rtk-skill` / `codegraph-skill`.

## Knowledge layers (mandatory order)

1. **Architecture** — `.terrain/agent/context.md`
   - Module map, core flows, system boundaries, tech stack
   - Check `.terrain/agent/context-meta.json` or `meta.json` for freshness
   - Check `.terrain/.meta/freshness.json` for drift score before trusting architecture context
   - Read directly (short); no RTK needed

2. **Private domain** — `.terrain/knowledge/**/*.md`
   - Business glossary, internal frameworks, APIs, scaffolding guides
   - Team-maintained markdown; read in filename sort order when surveying

3. **Structured meta** — `.terrain/agent/meta-inputs.md`
   - Compiled from `terrain-meta.json` and `knowledge/` scans

4. **Source index** — see `repomix-context-skill`
   - Local `.terrain/agent/repomix.md` (gitignored; regenerate via Terrain scan)

## Query workflow

```
Task received
  → Read context.md (or relevant section)
  → If business/internal terms → read knowledge/*.md
  → If symbol / call graph → codegraph-skill
  → If implementation / source → repomix-context-skill
  → If shell/git/test needed → rtk-skill (prefix with rtk)
```

## Rules

- **Do not** invent module names that contradict `context.md` or `meta-inputs.md`
- **Do not** read the entire live repository tree when indexed assets exist
- **Do not** load full `repomix.md` into context — grep slices only (`rtk grep` on the file is OK)
- Prefer `.terrain/` over guessing project structure

## Private knowledge directory

`.terrain/knowledge/` — developers add markdown here; Terrain scans on context generation.

Example files:
- `00-glossary.md` — domain terms
- `10-internal-framework.md` — internal libs
- `20-api-usage.md` — internal APIs
- `30-scaffolding.md` — project generators

## Human docs (optional)

`.terrain/human/` — Litho-generated docs for humans; useful for onboarding context but denser than `context.md`.

## Related skills

| Skill | When |
|-------|------|
| `repomix-context-skill` | Source code from repomix index |
| `codegraph-skill` | Callers, callees, impact |
| `rtk-skill` | All verbose shell commands |
