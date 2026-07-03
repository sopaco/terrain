---
name: rtk-skill
description: Use when running shell commands that produce verbose output (git, test, build, lint, package managers, docker). Prefix with rtk to save 60-90% tokens. Terrain projects use explicit rtk prefix (no global hook).
version: 1.2.0
---

# RTK Skill (Rust Token Killer)

[RTK](https://github.com/rtk-ai/rtk) is a CLI proxy that **filters and compresses command output** before it reaches the LLM (typically **60–90% token savings**).

## Resolve the RTK command (read first)

Use **conventional paths only** — never machine-specific absolute paths like `/Users/...` or `C:\Users\...`.

On Windows, tools deploy to `%USERPROFILE%\.terrain\bin\` (also written as `~/.terrain/bin/` in Git Bash / PowerShell 7+). Binaries use `.exe` extensions; PATHEXT resolves `rtk` → `rtk.exe`.

| Priority | Command prefix | When |
|----------|----------------|------|
| 1 | `~/.terrain/bin/rtk` | Terrain env integration or desktop app (see existence check below) |
| 2 | `bunx @terrain-ai/rtk` | No Terrain install; needs network once |
| 3 | `npx @terrain-ai/rtk` | Same as bunx if Bun unavailable |

Optional manifest (local, gitignored): `.terrain/env/agent-tools.json` — same `~/.terrain/bin/…` conventions.

**Existence check (cross-platform):**

| Shell | Check |
|-------|-------|
| bash / zsh / Git Bash | `[ -x ~/.terrain/bin/rtk ] \|\| [ -x ~/.terrain/bin/rtk.exe ]` |
| PowerShell | `Test-Path "$HOME\.terrain\bin\rtk.exe"` |
| cmd | `if exist "%USERPROFILE%\.terrain\bin\rtk.exe"` |

**Shell rules:**

- Invoke as `~/.terrain/bin/rtk <cmd>` — tilde expands at word start in bash/zsh/Git Bash/PowerShell 7+.
- Do **not** `export RTK="$(jq -r .rtk …)"` then `"$RTK"` — quoted variables do not expand `~`.
- Do **not** assume bare `rtk` is on PATH.

Example (pick one prefix per session after the existence check):

```bash
# bash / Git Bash
if [ -x ~/.terrain/bin/rtk ] || [ -x ~/.terrain/bin/rtk.exe ]; then
  PREFIX=~/.terrain/bin/rtk
else
  PREFIX="bunx @terrain-ai/rtk"
fi
$PREFIX git status
```

```powershell
# PowerShell
$PREFIX = if (Test-Path "$HOME\.terrain\bin\rtk.exe") { "$HOME\.terrain\bin\rtk.exe" } else { "bunx @terrain-ai/rtk" }
& $PREFIX git status
```

Verify:

```bash
~/.terrain/bin/rtk gain || bunx @terrain-ai/rtk gain
```

We do **not** run `rtk init` / global hooks.

## Golden rule

> For **any shell command** that prints more than a few lines, run it as **`<rtk-prefix> <cmd>`** instead of bare `<cmd>`.

Applies to: git, tests, builds, linters, package managers, docker/kubectl, `ls`/`grep`/`find`, `gh`, etc.

In examples below, `<rtk>` means your resolved prefix (`~/.terrain/bin/rtk` or `bunx @terrain-ai/rtk`).

## Critical: built-in Read / Grep / Glob

Claude Code, Cursor, and similar agents often have **native Read/Grep tools that bypass Bash hooks**.

Those tools **do not** auto-rewrite to RTK. For token-efficient file/search workflows, use:

| Instead of native tool | Use |
|------------------------|-----|
| Read large source file | `<rtk> read path/to/file.rs` |
| Read signatures only | `<rtk> read path/to/file.rs -l aggressive` |
| Grep / search repo | `<rtk> grep "pattern" .` or `<rtk> rg "pattern"` |
| Find files | `<rtk> find "*.ts" .` |
| List directory | `<rtk> ls .` |

**Exception:** `.terrain/agent/context.md` and short `knowledge/*.md` — read directly (already dense).

## Command reference (by category)

### Git (high savings)

```bash
<rtk> git status
<rtk> git log -n 20 --oneline
<rtk> git diff
<rtk> git diff --staged
<rtk> add -A && <rtk> git commit -m "msg"
<rtk> git push
<rtk> git pull
```

### Tests (failures-focused, ~90% savings)

```bash
<rtk> cargo test
<rtk> test cargo test
<rtk> bun test
<rtk> vitest
<rtk> jest
<rtk> pytest
<rtk> go test
```

### Build & lint

```bash
<rtk> cargo build
<rtk> cargo clippy
<rtk> tsc
<rtk> eslint .
<rtk> ruff check
<rtk> next build
```

### Package managers

```bash
<rtk> pnpm list
<rtk> bun install
<rtk> pip list
```

### Files & search

```bash
<rtk> ls src/
<rtk> read src/lib/foo.rs
<rtk> read src/lib/foo.rs -l aggressive
<rtk> grep "fn handle_" .
<rtk> find "*.svelte" .
<rtk> diff file1 file2
```

### Containers / cloud (when used)

```bash
<rtk> docker ps
<rtk> docker logs <container>
<rtk> kubectl get pods
<rtk> gh pr list
```

### Errors only

```bash
<rtk> err npm run build
```

## Global flags

```bash
<rtk> -u git status
<rtk> -v cargo test
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
RTK_DISABLED=1 git status
```

## When NOT to use RTK

| Use RTK | Use other Terrain skills instead |
|---------|----------------------------------|
| Shell command output | Architecture → `terrain-knowledge-skill` / `context.md` |
| git test build lint | Symbol relations → `codegraph-skill` |
| `<rtk> read` for code slices | Structured repomix index → `repomix-context-skill` |

**Workflow order:** Terrain knowledge → codegraph → repomix slices → **rtk** for remaining shell work.

## Analytics (optional)

```bash
<rtk> gain
<rtk> gain --history
<rtk> discover
```

## Do not

- Run `rtk init` or `rtk init -g` — Terrain owns agent guidance via AGENTS.md
- Assume hooks rewrite commands — **you** must type the rtk prefix
- Re-run identical verbose commands after RTK already summarized them
- Use wrong rtk package (verify with `<rtk> gain`)

## Complements

- **terrain-knowledge-skill** — what project knowledge to read first
- **codegraph-skill** — AST/call-graph queries
- **repomix-context-skill** — grep repomix pack for source sections
