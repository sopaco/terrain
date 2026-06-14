---
name: codegraph-skill
description: Use when a coding agent needs symbol relationships, callers, callees, or change impact. Guides Codegraph CLI usage (not MCP).
version: 1.1.0
---

# Codegraph Skill

[Codegraph](https://colbymchenry.github.io/codegraph/) provides a pre-indexed **AST code graph** for this project.

MindMesh uses **CLI only** — do not run `codegraph install` (that configures MCP/agent rules separately).

## Prerequisites

```bash
bunx codegraph status    # must succeed
```

If not initialized:

```bash
bun add -d @colbymchenry/codegraph
bunx codegraph init -i
```

Index lives in `.codegraph/` (regenerate with `bunx codegraph sync` after edits).

## CLI commands

Always run via `bunx codegraph …` (or `codegraph` on PATH):

| Intent | Command |
|--------|---------|
| Find symbol by name | `bunx codegraph query <name>` |
| Who calls X | `bunx codegraph callers <symbol>` |
| What X calls | `bunx codegraph callees <symbol>` |
| Change blast radius | `bunx codegraph impact <symbol>` |
| Tests affected by file changes | `bunx codegraph affected <files…>` |
| Project file tree | `bunx codegraph files` |
| Index health | `bunx codegraph status` |
| Refresh after edits | `bunx codegraph sync` |

## Recommended workflow

1. Read `.mind-mesh/agent/context.md` (`mind-mesh-knowledge-skill`)
2. `bunx codegraph query <SymbolName>` to locate definition
3. `callers` / `callees` / `impact` for relationship questions
4. `repomix-context-skill` for full source text of a specific file
5. Use **`rtk-skill`** for any follow-up shell commands (tests, git)

## When to use vs other skills

| Use Codegraph | Use instead |
|---------------|-------------|
| Symbol lookup, call chains | Architecture → `context.md` |
| Impact before refactor | Business rules → `knowledge/` |
| File/symbol graph | Raw source slice → repomix |
| Verbose test/git output | `rtk cargo test`, `rtk git diff` |

## Do not

- Run `codegraph install` (MindMesh manages AGENTS.md)
- Blind `grep` the whole repo to re-verify Codegraph AST results
- Chain `query` + manual file reads when `impact` answers the question

## Staleness

If `codegraph status` shows pending files after your edits:

```bash
bunx codegraph sync
```
