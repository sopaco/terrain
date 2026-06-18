---
name: codegraph-skill
description: Use when a coding agent needs symbol relationships, callers, callees, or change impact. Guides CodeGraph CLI usage (not MCP).
version: 1.1.0
---

# CodeGraph Skill

[CodeGraph](https://colbymchenry.github.io/codegraph/) provides a pre-indexed **AST code graph** for this project.

MindMesh uses **CLI only** — do not run `codegraph install` (that configures MCP/agent rules separately).

## Prerequisites

MindMesh deploys CodeGraph to **`~/.mind-mesh/bin/codegraph`** (symlink to bundled runtime). The **index** is per-repo under `.codegraph/`.

**Read executable paths first:**

```bash
cat .mind-mesh/env/agent-tools.json
```

```bash
# health check (use absolute path from manifest)
~/.mind-mesh/bin/codegraph status
```

If the repo has no index yet, run env integration or:

```bash
~/.mind-mesh/bin/codegraph init -i
```

Index lives in `.codegraph/` (refresh with `codegraph sync` after edits).

## CLI commands

Use the **`codegraph` absolute path** from `agent-tools.json` (or `~/.mind-mesh/bin/codegraph`):

| Intent | Command |
|--------|---------|
| Find symbol by name | `codegraph query <name>` |
| Who calls X | `codegraph callers <symbol>` |
| What X calls | `codegraph callees <symbol>` |
| Change blast radius | `codegraph impact <symbol>` |
| Tests affected by file changes | `codegraph affected <files…>` |
| Project file tree | `codegraph files` |
| Index health | `codegraph status` |
| Refresh after edits | `codegraph sync` |

## Recommended workflow

1. Read `.mind-mesh/agent/context.md` (`mind-mesh-knowledge-skill`)
2. `codegraph query <SymbolName>` to locate definition
3. `callers` / `callees` / `impact` for relationship questions
4. `repomix-context-skill` for full source text of a specific file
5. Use **`rtk-skill`** for any follow-up shell commands (tests, git)

## When to use vs other skills

| Use CodeGraph | Use instead |
|---------------|-------------|
| Symbol lookup, call chains | Architecture → `context.md` |
| Impact before refactor | Business rules → `knowledge/` |
| File/symbol graph | Raw source slice → repomix |
| Verbose test/git output | `rtk cargo test`, `rtk git diff` |

## Do not

- Run `codegraph install` (MindMesh manages AGENTS.md)
- Blind `grep` the whole repo to re-verify CodeGraph AST results
- Chain `query` + manual file reads when `impact` answers the question

## Staleness

If `codegraph status` shows pending files after your edits:

```bash
codegraph sync
```
