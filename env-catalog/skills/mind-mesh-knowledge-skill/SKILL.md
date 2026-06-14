---
name: mind-mesh-knowledge-skill
description: Use when a coding agent needs project knowledge from MindMesh .mind-mesh/ assets. Guides layered reading of context, private knowledge, and repomix index.
version: 1.1.0
---

# MindMesh Knowledge Skill

MindMesh stores **AI knowledge assets** under **`.mind-mesh/`** in this repository (not a global `~/.mind-mesh/knowledge` directory). The desktop app registry at `~/.mind-mesh/registry.json` only maps slugs → repo paths.

Load **`rtk-skill`** when you need to run shell commands during investigation (git, grep repomix file, tests).

## Knowledge layers (mandatory order)

1. **Architecture** — `.mind-mesh/agent/context.md`
   - Module map, core flows, system boundaries, tech stack
   - Check `.mind-mesh/agent/context-meta.json` or `meta.json` for freshness
   - Read directly (short); no RTK needed

2. **Private domain** — `.mind-mesh/knowledge/**/*.md`
   - Business glossary, internal frameworks, APIs, scaffolding guides
   - Team-maintained markdown; read in filename sort order when surveying

3. **Structured meta** — `.mind-mesh/agent/meta-inputs.md`
   - Compiled from `mind-mesh-meta.json` and `knowledge/` scans

4. **Source index** — see `repomix-context-skill`
   - Local `.mind-mesh/agent/repomix.md` (gitignored; regenerate via MindMesh scan)

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
- Prefer `.mind-mesh/` over guessing project structure

## Private knowledge directory

`.mind-mesh/knowledge/` — developers add markdown here; MindMesh scans on context generation.

Example files:
- `00-glossary.md` — domain terms
- `10-internal-framework.md` — internal libs
- `20-api-usage.md` — internal APIs
- `30-scaffolding.md` — project generators

## Human docs (optional)

`.mind-mesh/human/` — Litho-generated docs for humans; useful for onboarding context but denser than `context.md`.

## Related skills

| Skill | When |
|-------|------|
| `repomix-context-skill` | Source code from repomix index |
| `codegraph-skill` | Callers, callees, impact |
| `rtk-skill` | All verbose shell commands |
