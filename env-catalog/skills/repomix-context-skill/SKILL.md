---
name: repomix-context-skill
description: Use when an agent needs source code from the local repomix index under .mind-mesh/agent/repomix.md (not committed; regenerate via MindMesh scan).
version: 1.2.0
---

# Repomix Context Skill

MindMesh stores a **local repomix index** at `.mind-mesh/agent/repomix.md` (gitignored, fast to regenerate).

Read **architecture first** via `mind-mesh-knowledge-skill` → `.mind-mesh/agent/context.md`.

## When to use

- Implementation details after reading `context.md`
- Cross-file symbol search within the indexed snapshot
- Locating handlers, types, routes

## Query strategy (mandatory)

1. **Read meta** — `.mind-mesh/agent/meta.json` (`total_tokens`, `synced_at`, `top_files_by_tokens`)
2. **Search the pack** — never load the entire file:
   ```bash
   rtk grep "struct ProjectOverview" .mind-mesh/agent/repomix.md
   rtk grep "### src/lib/api.ts" .mind-mesh/agent/repomix.md
   ```
   Or agent Grep limited to that path with tight patterns.
3. **Read slices** — extract matching `### path/to/file` sections only:
   ```bash
   rtk read .mind-mesh/agent/repomix.md -l aggressive   # scan structure first if huge
   ```
   Then read the specific `### file` block (line range), ≤150 lines per read.
4. **Refresh** — if `meta.json.synced_at` is stale, ask user to run MindMesh **重建源码索引** / scan

## Repomix section format

Sections look like:

```markdown
### src/lib/foo.ts

\`\`\`typescript
... file content ...
\`\`\`
```

Grep for `### relative/path` to jump to a file.

## Paths

| File | Purpose |
|------|---------|
| `.mind-mesh/agent/repomix.md` | Full indexed snapshot (local only) |
| `.mind-mesh/agent/meta.json` | Pack metrics |
| `.mind-mesh/agent/context.md` | Architecture (read first) |

## Do not

- Commit or assume `repomix.md` exists in git
- `cat` / Read the entire `repomix.md` (can be 100k+ tokens)
- Read the live repository tree when the pack covers the question
- Skip `context.md` and grep source for architecture questions

## Regenerate index

If `repomix.md` is missing:

```bash
mind-mesh assets pack-agent <repo-path>
# or from repo root in MindMesh UI: 重建源码索引
```

## Related skills

- **codegraph-skill** — symbol relationships (prefer before wide repomix grep)
- **rtk-skill** — use `rtk grep` / `rtk read` on the pack file
