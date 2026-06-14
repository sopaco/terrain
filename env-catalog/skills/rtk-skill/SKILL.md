---
name: rtk-skill
description: Use when running shell commands that produce verbose output (git, test, build, lint, package managers, docker). Prefix with rtk to save 60-90% tokens. MindMesh projects use explicit rtk prefix (no global hook).
version: 1.1.0
---

# RTK Skill (Rust Token Killer)

[RTK](https://github.com/rtk-ai/rtk) is a CLI proxy that **filters and compresses command output** before it reaches the LLM (typically **60–90% token savings**).

## MindMesh setup (this repo)

MindMesh integrates RTK via **AGENTS.md + this skill** — we do **not** run `rtk init` / global hooks.

**You must prefix commands explicitly:**

```bash
rtk <original-command-and-args>
```

Resolve binary (try in order):

1. `rtk` on PATH (common in dev environments)
2. `bunx rtk` (project-local via `@mind-mesh/rtk`)
3. `./node_modules/.bin/rtk`

Verify:

```bash
rtk gain          # must print savings stats (not "command not found")
rtk --version     # must show rtk-ai/rtk, NOT Rust Type Kit from crates.io
```

## Golden rule

> For **any shell command** that prints more than a few lines, run it as **`rtk <cmd>`** instead of bare `<cmd>`.

Applies to: git, tests, builds, linters, package managers, docker/kubectl, `ls`/`grep`/`find`, `gh`, etc.

## Critical: built-in Read / Grep / Glob

Claude Code, Cursor, and similar agents often have **native Read/Grep tools that bypass Bash hooks**.

Those tools **do not** auto-rewrite to RTK. For token-efficient file/search workflows, use:

| Instead of native tool | Use |
|------------------------|-----|
| Read large source file | `rtk read path/to/file.rs` |
| Read signatures only | `rtk read path/to/file.rs -l aggressive` |
| Grep / search repo | `rtk grep "pattern" .` or `rtk rg "pattern"` |
| Find files | `rtk find "*.ts" .` |
| List directory | `rtk ls .` |

**Exception:** `.mind-mesh/agent/context.md` and short `knowledge/*.md` — read directly (already dense).

## Command reference (by category)

### Git (high savings)

```bash
rtk git status
rtk git log -n 20 --oneline
rtk git diff
rtk git diff --staged
rtk add -A && rtk git commit -m "msg"   # commit/push often collapse to "ok ..."
rtk git push
rtk git pull
```

### Tests (failures-focused, ~90% savings)

```bash
rtk cargo test
rtk test cargo test          # generic wrapper — failures only
rtk bun test                 # or: rtk npm test / rtk pnpm test
rtk vitest
rtk jest
rtk pytest
rtk go test
```

### Build & lint

```bash
rtk cargo build
rtk cargo clippy
rtk tsc
rtk eslint .                 # or: rtk lint
rtk ruff check
rtk next build
```

### Package managers

```bash
rtk pnpm list
rtk bun install
rtk pip list
```

### Files & search

```bash
rtk ls src/
rtk read src/lib/foo.rs
rtk read src/lib/foo.rs -l aggressive   # signatures only
rtk grep "fn handle_" .
rtk find "*.svelte" .
rtk diff file1 file2
```

### Containers / cloud (when used)

```bash
rtk docker ps
rtk docker logs <container>
rtk kubectl get pods
rtk gh pr list
```

### Errors only

```bash
rtk err npm run build        # stderr / errors from any command
```

## Global flags

```bash
rtk -u git status            # --ultra-compact (extra compression)
rtk -v cargo test            # more verbose when debugging
```

## When command fails

RTK may collapse output but preserves **exit codes**. On failure, look for a tee path in output:

```
FAILED: 2/15 tests
[full output: ~/.local/share/rtk/tee/....log]
```

Read that log if you need the full unfiltered output — do not blindly re-run the same verbose command.

## Bypass RTK (rare)

```bash
RTK_DISABLED=1 git status    # one-off full output
```

## When NOT to use RTK

| Use RTK | Use other MindMesh skills instead |
|---------|-----------------------------------|
| Shell command output | Architecture → `mind-mesh-knowledge-skill` / `context.md` |
| git test build lint | Symbol relations → `codegraph-skill` |
| `rtk read` for code slices | Structured repomix index → `repomix-context-skill` |

**Workflow order:** MindMesh knowledge → codegraph → repomix slices → **rtk** for remaining shell work.

## Analytics (optional)

```bash
rtk gain                     # session savings summary
rtk gain --history           # recent commands
rtk discover                 # missed optimization opportunities
```

## Do not

- Run `rtk init` or `rtk init -g` — MindMesh owns agent guidance via AGENTS.md
- Assume hooks rewrite commands — **you** must type `rtk` prefix
- Re-run identical verbose commands after RTK already summarized them
- Use wrong `rtk` package (verify with `rtk gain`)

## Complements

- **mind-mesh-knowledge-skill** — what project knowledge to read first
- **codegraph-skill** — AST/call-graph queries
- **repomix-context-skill** — grep repomix pack for source sections
