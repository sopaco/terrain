---
name: codegraph-skill
description: Use when a coding agent needs symbol relationships, callers, callees, or change impact. Guides CodeGraph CLI usage (not MCP).
version: 1.3.0
---

# CodeGraph Skill

[CodeGraph](https://colbymchenry.github.io/codegraph/) provides a pre-indexed **AST code graph** for this project.

Terrain uses **CLI only** — do not run `codegraph install` (that configures MCP/agent rules separately).

## Resolve the CodeGraph command (read first)

Use **conventional paths only** — never machine-specific absolute paths like `/Users/...` or `C:\Users\...`.

On Windows, the wrapper may be `codegraph.cmd` or `codegraph.exe` under `~/.terrain/bin/`.

| Priority | Command | When |
|----------|---------|------|
| 1 | `~/.terrain/bin/codegraph` | Terrain env integration (see existence check below) |
| 2 | `bunx codegraph` | No Terrain install; needs network once |
| 3 | `npx codegraph` | Same as bunx if Bun unavailable |

Optional manifest (local, gitignored): `.terrain/env/agent-tools.json` — same `~/.terrain/bin/…` conventions.

**Existence check (cross-platform):**

| Shell | Check |
|-------|-------|
| bash / zsh / Git Bash | `[ -x ~/.terrain/bin/codegraph ] \|\| [ -f ~/.terrain/bin/codegraph.exe ] \|\| [ -f ~/.terrain/bin/codegraph.cmd ]` |
| PowerShell | `Test-Path "$HOME\.terrain\bin\codegraph*"` |

In examples below, `<cg>` means your resolved prefix (`~/.terrain/bin/codegraph` or `bunx codegraph`).

## Prerequisites

The **index** is per-repo under `.codegraph/`. If missing, initialize from the repo root:

```bash
# bash / Git Bash
if [ -x ~/.terrain/bin/codegraph ] || [ -f ~/.terrain/bin/codegraph.exe ] || [ -f ~/.terrain/bin/codegraph.cmd ]; then
  ~/.terrain/bin/codegraph init -i
else
  bunx codegraph init -i
fi
```

Refresh after edits: `<cg> sync`

Health check:

```bash
<cg> status
```

**`<cg> status` can lie ("up to date" when it is not).** Observed case: index untouched for 10 days while 24 commits changed source files, `status` still reported fresh, and `<cg> query` on a symbol added days earlier returned nothing. Do not treat `status` as authoritative on its own.

## CLI commands

| Intent | Command |
|--------|---------|
| Find symbol by name | `<cg> query <name>` |
| Who calls X | `<cg> callers <symbol>` |
| What X calls | `<cg> callees <symbol>` |
| Change blast radius | `<cg> impact <symbol>` |
| Tests affected by file changes | `<cg> affected <files…>` |
| Project file tree | `<cg> files` |
| Index health | `<cg> status` |
| Refresh after edits | `<cg> sync` |

## Recommended workflow

1. Read `.terrain/agent/context.md` (`terrain-knowledge-skill`)
2. **Always** `<cg> sync` first — cheap, incremental, and removes the need to trust `status`'s own up-to-date claim
3. `<cg> query <SymbolName>` to locate definition
4. `callers` / `callees` / `impact` for relationship questions
5. `repomix-context-skill` for full source text of a specific file
6. Use **`rtk-skill`** for any follow-up shell commands (tests, git)

## Independent staleness cross-check (mandatory before impact/callers analysis)

Because `<cg> status` is not reliable, cross-check with Terrain's own git-based drift report before trusting `impact`/`callers`/`callees` results for anything safety-relevant (e.g. "is it safe to remove this function"):

```bash
~/.terrain/bin/terrain tools codegraph-drift --project <slug>
# or: bunx @terrain-ai/cli tools codegraph-drift --project <slug>
```

This compares `.codegraph/codegraph.db`'s mtime against `git log --since` — independent of CodeGraph's own bookkeeping. If `likely_stale: true`, run `<cg> sync` and re-check before trusting query results.

## When to use vs other skills

| Use CodeGraph | Use instead |
|---------------|-------------|
| Symbol lookup, call chains | Architecture → `context.md` |
| Impact before refactor | Business rules → `knowledge/` |
| File/symbol graph | Raw source slice → repomix |
| Verbose test/git output | `<rtk> cargo test`, `<rtk> git diff` |

## Do not

- Run `codegraph install` (Terrain manages AGENTS.md)
- Blind `grep` the whole repo to re-verify CodeGraph AST results
- Chain `query` + manual file reads when `impact` answers the question

## Staleness

If `<cg> status` shows pending files after your edits, or `terrain tools codegraph-drift` reports `likely_stale: true`:

```bash
<cg> sync
```
