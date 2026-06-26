---
name: repomix-context-skill
description: Use when an agent needs source code from the local repomix index under .terrain/agent/repomix.md (not committed; regenerate via Terrain scan).
version: 1.3.0
---

# Repomix Context Skill

Terrain stores a **local repomix index** at `.terrain/agent/repomix.md` (gitignored, fast to regenerate).

Read **architecture first** via `terrain-knowledge-skill` → `.terrain/agent/context.md`.

## When to use

- Implementation details after reading `context.md`
- Cross-file symbol search within the indexed snapshot
- Locating handlers, types, routes

## Query strategy (mandatory)

1. **Read meta** — `.terrain/agent/meta.json` (`total_tokens`, `synced_at`, `top_files_by_tokens`)
2. **Search the pack** — never load the entire file:
   ```bash
   # <rtk> = ~/.terrain/bin/rtk or bunx @terrain-ai/rtk (see rtk-skill)
   <rtk> grep "struct ProjectOverview" .terrain/agent/repomix.md
   <rtk> grep "### src/lib/api.ts" .terrain/agent/repomix.md
   ```
   Or agent Grep limited to that path with tight patterns.
3. **Read slices** — extract matching `### path/to/file` sections only:
   ```bash
   <rtk> read .terrain/agent/repomix.md -l aggressive
   ```
   Then read the specific `### file` block (line range), ≤150 lines per read.
4. **Refresh** — if `meta.json.synced_at` is stale, ask user to run Terrain **重建源码索引** / scan

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
| `.terrain/agent/repomix.md` | Full indexed snapshot (local only) |
| `.terrain/agent/meta.json` | Pack metrics |
| `.terrain/agent/context.md` | Architecture (read first) |

## Do not

- Commit or assume `repomix.md` exists in git
- `cat` / Read the entire `repomix.md` (can be 100k+ tokens)
- Read the live repository tree when the pack covers the question
- Skip `context.md` and grep source for architecture questions

## Regenerate index

If `repomix.md` is missing:

```bash
# bash / Git Bash — <terrain> = ~/.terrain/bin/terrain or bunx @terrain-ai/cli
if [ -x ~/.terrain/bin/terrain ] || [ -x ~/.terrain/bin/terrain.exe ]; then
  ~/.terrain/bin/terrain assets pack-agent .
else
  bunx @terrain-ai/cli assets pack-agent .
fi
# or from repo root in Terrain UI: 重建源码索引
```

If neither `~/.terrain/bin/terrain` nor `bunx @terrain-ai/cli` works, ask the user to install Terrain or run **重建源码索引** from the desktop app.

## Related skills

- **codegraph-skill** — symbol relationships (prefer before wide repomix grep)
- **rtk-skill** — resolve `<rtk>` prefix for grep/read on the pack file
