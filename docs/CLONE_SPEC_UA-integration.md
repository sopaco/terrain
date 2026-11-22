# CLONE_SPEC — Understand-Anything (UA) visual + tools integration into Terrain

Source studied: https://github.com/Egonex-AI/Understand-Anything (MIT, public source,
v2.9.8 at `/Users/carolinafogliato/Github/Understand-Anything`) via the REAP 04
method: extract *behavior* with evidence, rebuild inside Terrain's own architecture
(Rust commands + Svelte 5 panels + env-catalog packaging). No UA code is vendored;
the only UA artifacts Terrain consumes are its MIT-licensed runtime processes
(skill pipeline invoked by the agent, viewer server for the dashboard).

## Evidence (UA repo, file:line per REAP readers)

| # | Behavior | Evidence |
|---|----------|----------|
| E1 | UA is a Claude Code plugin, not a library: manifest registers skills/ + agents/ + hooks, no MCP servers, no bin. Everything is slash-skill-driven | `understand-anything-plugin/.claude-plugin/plugin.json` — "Manifest {name: understand-anything, version 2.9.8} registers three surfaces: skills/ (slash commands), agents/ (subagent definitions …), hooks/hooks.json" |
| E2 | 8 user-invocable skills: `/understand` (7-phase pipeline), `/understand-dashboard`, `/understand-chat`, `/understand-diff`, `/understand-domain`, `/understand-explain`, `/understand-figma`, `/understand-knowledge`, `/understand-onboard` | `understand-anything-plugin/skills/` — "Eight skills with SKILL.md YAML frontmatter: /understand [path [--full\|--auto-update\|--review\|--language <lang>\|--exclude <patterns>]] — 7-phase pipeline writing .ua/knowledge-graph.json" |
| E3 | 10 agent definitions dispatched as Task subagents by the /understand orchestration (project-scanner, file-analyzer, architecture-analyzer, graph-reviewer, tour-builder, domain-analyzer, article-analyzer, assemble-reviewer, design-analyzer, knowledge-graph-guide) | `understand-anything-plugin/agents/` — "Ten agent .md files dispatched via Task subagents by the orchestration in SKILL.md, not user-invoked … file-analyzer.md (Phase 2, per-batch tree-sitter extract-structure.mjs run + LLM semantic analysis, up to 5 concurrent)" |
| E4 | The acting assistant IS the LLM — no API client, no keys (except FIGMA_TOKEN for the figma skill); programmatic surface is only prompt construction | `understand-anything-plugin/packages/core/src/analyzer/llm-analyzer.ts` — "zero API-key/provider/HTTP references … The acting assistant is the LLM" |
| E5 | Pipeline output dir: `.ua/` at project root, legacy `.understand-anything/` wins for reads AND writes when it exists; git-worktree redirect moves output to the main repo checkout unless `UNDERSTAND_NO_WORKTREE_REDIRECT=1` | `understand-anything-plugin/skills/understand/SKILL.md` — "resolveUaDir: legacy `.understand-anything/` wins for reads AND writes … if PROJECT_ROOT is a worktree, output goes to the main repo root" |
| E6 | Final artifacts: `knowledge-graph.json` (nodes/edges/layers/tour, zod-validated `KnowledgeGraphSchema`), `meta.json` {lastAnalyzedAt, gitCommitHash, version, analyzedFiles}, `config.json` {autoUpdate, outputLanguage?}, `fingerprints.json` baseline; subdomain graphs `*-knowledge-graph.json` merged by merge-subdomain-graphs.py | `understand-anything-plugin/skills/understand/SKILL.md` — "meta.json … written only after fingerprints succeed, else auto-update would classify everything STRUCTURAL" |
| E7 | Intermediate state: `.ua/intermediate/{scan-result.json, batches.json, batch-<i>.json, assembled-graph.json, …}`; cleanup moves everything except scan-result.json into `.ua/.trash-<epoch>`, purged after 7 days, never `rm -rf` | `understand-anything-plugin/skills/understand/` — "Cleanup: everything except scan-result.json (preserved to skip Phase 1 re-scan) is mv'd into .ua/.trash-<epoch>" |
| E8 | 7 phases: 0 pre-flight/incremental-decision → 1 SCAN → 1.5 BATCH → 2 ANALYZE (tree-sitter + LLM per batch) → 3 layers → 4 review → 5 tour → 6 finalize (atomic save, one symbol-retry) → 7 fingerprints + meta + optional dashboard autolaunch | `understand-anything-plugin/skills/understand/SKILL.md` — "2 ANALYZE (file-analyzer subagent per batch, 5 concurrent: runs extract-structure.mjs … then LLM analysis … merge-batch-graphs.py -> assembled-graph.json)" |
| E9 | Incremental auto-update is hook-driven: PostToolUse (matcher `Bash`, regex `git (commit|merge|cherry-pick|rebase)`) and SessionStart (meta.json gitCommitHash != `git rev-parse HEAD`); both emit additionalContext telling the assistant to execute `hooks/auto-update-prompt.md` — the update itself is agent-executed prose, not a script | `understand-anything-plugin/hooks/hooks.json` — "gates on autoUpdate===true in config.json and existence of knowledge-graph.json, then emits hookSpecificOutput additionalContext" |
| E10 | Dashboard: one React 19 + Vite 6 + React Flow 12 app, no router; mode-state machine (TokenGate → Dashboard) switched by zustand `viewMode` over three canvases (structural GraphView, DomainGraphView, KnowledgeGraphView); ELK/dagre/d3-force layouts in web workers, louvain community clustering; fuzzy/semantic search, persona tour, filters, export, diff overlay, path finder | `understand-anything-plugin/packages/dashboard/src/App.tsx` (~97) + `src/store.ts` (~101) — "Three React Flow canvases switched by useDashboardStore viewMode"; "ALL_NODE_TYPES = 27 types; ALL_EDGE_CATEGORIES = 9" |
| E11 | Dashboard data contract is runtime-only fetches, root-relative: `knowledge-graph.json` (required), `domain-graph.json`, `diff-overlay.json`, `meta.json`, `config.json`, `staleness.json` (polled), `file-content.json?path=<rel>&token=<t>` — endpoints exist only in the vite dev middleware or the viewer server, which relativize node filePaths and whitelist file reads against graph nodes (1 MB cap, no binary) | `understand-anything-plugin/packages/dashboard/src/App.tsx` (~144, `dataUrl()` ~56) — "server-side JSON discovery: GRAPH_DIR env first …; per root tries .understand-anything/ (legacy) then .ua/" |
| E12 | Graph validation is tolerant tiered validateGraph: sanitize → normalize type aliases → auto-fix (invalid nodes/edges dropped with per-item GraphIssues) → fatal only on schema-shape failures; non-fatal issues render in a WarningBanner phrased for the agent to fix the graph | `understand-anything-plugin/packages/core/src/schema.ts` (~563) — "invalid nodes DROPPED with GraphIssue level 'dropped' … fatal on … zero valid nodes" |
| E13 | The static dist alone cannot show a real project: it ships a SAMPLE graph in `public/knowledge-graph.json`, so bare static hosting silently renders demo data. Two serving modes: non-demo dist at a server root with `?token=`, or demo build (base `/demo/`, `VITE_DEMO_MODE`, `VITE_*_URL` envs, token gate bypassed) | `understand-anything-plugin/packages/dashboard/vite.config.ts` (~345, dev middleware) + `vite.config.demo.ts` (~6) — "public/knowledge-graph.json (99KB sample graph) … a bare static server returns 200 for /knowledge-graph.json and the dashboard self-renders the SAMPLE graph" |
| E14 | Embedding via srcdoc breaks today: asset URLs are baked absolute at build time, data fetches are root-relative, and App.tsx touches `sessionStorage`/`localStorage`/`history.replaceState` with no guards (SecurityError on opaque origins → blank app). Token must arrive via the iframe `?token=` URL | `understand-anything-plugin/packages/dashboard/src/App.tsx` (~80–95) — "persists UI state with NO try/catch guards … In an opaque (srcdoc/null-origin) … context these can throw SecurityError and blank the app" |
| E15 | Assistant-free viewing: npm bin `understand-anything-viewer` (packages/viewer, Node ≥ 18) — zero-dependency http server mirroring the vite middleware, binds 127.0.0.1, one-time `?token=`, serves relativized graph + whitelisted file-content, auto-opens browser, prints the dashboard URL with token | `understand-anything-plugin/packages/viewer/bin/viewer.mjs` — "serve a generated knowledge graph in the dashboard UI with nothing but Node.js. Read-only, no Claude Code, no LLM" |
| E16 | Install is platform-branched: install.sh's 14-row symlink table (per-skill vs folder style, e.g. gemini/codex → `~/.agents/skills`, cline → folder `~/.cline/skills/understand-anything`) does NOT include claude-code — Claude Code installs natively via `/plugin marketplace add Egonex-AI/Understand-Anything` + `/plugin install understand-anything`; all non-CC platforms also get one universal `~/.understand-anything-plugin` symlink | `install.sh` (~25 platform table, ~172 universal link) + `README.md` (~189) — "install.sh does NOT support claude-code at all — Claude Code is native-plugin only" |
| E17 | Uninstall/update: `--update` = `git pull --ff-only` on the repo checkout (symlinks keep working); uninstall removes the universal symlink and sweeps stale per-skill links; uninstalling ANY platform removes the shared `~/.understand-anything-plugin` symlink other platforms still use | `install.sh` (~260) — "Always: removes $HOME/.understand-anything-plugin only if it is a symlink" |
| E18 | Hard requirements: Node ≥ 18, pnpm workspace with `@understand-anything/core` built (`packages/core/dist/index.js` gates Phase 1.5), Python 3 (merge-batch-graphs.py, merge-subdomain-graphs.py), tree-sitter grammars as npm/WASM deps (10 languages extracted; Swift/Kotlin/shell/PowerShell fall back to metrics + regex) | `understand-anything-plugin/packages/core/package.json` — "Phase 1.5 gates on existence of packages/core/dist/index.js because scripts import @understand-anything/core" |
| E19 | Cost/staleness model: staleness = git commit hash comparison vs meta.json; >100 files triggers user confirmation; ~157k tokens per re-scan without preserved scan-result.json | `understand-anything-plugin/hooks/hooks.json` (E9) + skills/understand/SKILL.md — "skill warns >100 files needs user confirmation, and per-run cost notes (~157k tokens per re-scan …)" |
| E20 | UA has no headless analysis entrypoint: only the deterministic scripts (scan/batch/extract/merge/fingerprints) run without an assistant; reuse contract for Terrain = the `.ua/` file shapes + core exports (PromptBuilders, validateGraph, TreeSitterPlugin, staleness) | `understand-anything-plugin/packages/core/package.json` — "UA is an assistant-driven skill pipeline, not a library" |

## Requirements derived for Terrain (own architecture)

1. **Analysis stays agent-driven** — Terrain does not port or wrap the LLM
   stages. A Terrain-authored skill (`understand-skill`, see Setup) instructs the
   coding agent to run UA's `/understand` pipeline when the UA plugin is present,
   with outputs directed to `.terrain/understand/` (R2 data-dir policy below).
   No new Rust LLM pipeline for graph analysis — graphify already covers
   AST-only graphing; UA adds the LLM-semantic layer on top, opt-in and
   cost-flagged (E19).
2. **Data-dir policy** — UA outputs (knowledge-graph.json, meta.json,
   config.json, fingerprints.json, intermediate/) are archived under
   `.terrain/understand/` to keep the memory bank single-rooted. UA's own
   resolver (E5) is fixed, so Terrain reads BOTH conventions: probe
   `.terrain/understand/` first, then `.ua/`, then legacy
   `.understand-anything/`. On write, if UA cannot be pointed at
   `.terrain/understand/` (UNKNOWN-U1), the skill writes `.ua/` and archives
   into `.terrain/understand/` on finish; a `gitignore`-kind env integration
   adds `/.ua/` to the repo root .gitignore.
3. **Status + staleness with Terrain's own command** — `ua_status_cmd(repo)`
   in `src-tauri/src/commands/` (graphify.rs pattern): presence flags
   (meta.json, knowledge-graph.json, fingerprints.json), node/link counts with
   the existing large-JSON caps, `staleness_of(meta.gitCommitHash, head)`
   reusing terrain-dashboard's staleness helper, plus UA-availability flags
   (plugin present? node ≥ 18? viewer reachable?).
4. **Run command** — `run_ua_cmd(app, repo, project_slug, action)` clones
   `run_graphify_cmd`: validate_repo → spawn the UA driver → stream
   `ua-progress` / `ua-done` events (namespaced events + a `uaBusy` field on
   projectTasks; do NOT reuse graphify's busy fields/event names — cross-talk
   risk). Action `"analyze"` drives the assistant-run skill phases; Terrain
   surfaces progress lines and completion.
5. **Dashboard serving = UA viewer process, not srcdoc** — UA's dashboard
   requires its companion fetch endpoints (E11, E13, E14): srcdoc with
   `sandbox="allow-scripts"` (opaque origin) cannot fetch them and its
   unguarded storage calls throw. Terrain spawns `understand-anything-viewer
   <project-dir>` per repo (terrain-dashboard runner pattern, matching
   resolve_graphify_bin; resolution: explicit path env → built checkout via
   packages/viewer → `npx` tarball fast path → error with install hint),
   parses the printed `Dashboard URL: http://127.0.0.1:<port>?token=<token>`
   line, and the UA view is an iframe with `src` (real http origin — the one
   embed mode UA is built for), not srcdoc. Sandbox needs
   `allow-scripts allow-same-origin` (isolated 127.0.0.1 origin) so UA's
   sessionStorage persistence (E14) does not throw.
6. **View switcher inside the graph panel** — GraphifyPanel gains
   `activeView: 'graphify' | 'ua'`: graphify keeps the existing srcdoc iframe +
   16MB protocol unchanged; the UA view renders its own iframe with the viewer
   URL. Per-view state is keyed (viewHtml stays graphify's; uaUrl for UA) and
   the staleness chip row is reused per active view (graphify staleness vs
   ua_status staleness). No new AppTab in v1 (panel-local switcher).
7. **No path-allowlist expansion** — Terrain does not gain any endpoint that
   reads repo paths chosen by the frontend. UA's viewer server does its own
   whitelisting from inside its process; Terrain only stores/returns the URL.
   The `get_graphify_view_cmd` artifact-selector generalization is deferred:
   UA's view does not use srcdoc, so no second single-file artifact exists yet.
8. **Validation surfacing** — UA's Dashboard renders its own validation tiers
   (E12); Terrain does not re-validate graphs. WarningBanner content lives
   inside UA's app; Terrain's panel only shows lifecycle issues of its own
   (viewer not running, URL/token parse failure, staleness).
9. **i18n** — every new user-facing string lands in `en/graph.ts` and
   `zh-CN/graph.ts` in the typed `as const` style.
10. **Version pinning** — env-catalog pins the UA version it was built against
    (v2.9.8 at spec time) in install steps and the viewer npx URL; no
    capability negotiation with unknown future versions.

## UI contract (Terrain tokens)

- GraphifyPanel header: title unchanged; new view pills (Graphify |
  Understand) in the same button styling as other panel chrome.
- UA view states, mirroring the panel's existing branches:
  - toolchain unavailable → install card (pre block `npx
    understand-anything-viewer` + plugin install steps), same treatment as
    graphify's `binary_available` card.
  - viewer running → iframe `src={uaUrl}` (token in URL); no srcdoc.
  - no graph → dashed empty-state card with an Analyze button (drives the
    skill flow), same treatment as graphify's Generate card.
  - staleness chip: existing colors (bg-tr-good-soft / bg-tr-watch-soft /
    bg-tr-elevated) fed from ua_status staleness.
  - busy/progress: spinner + progress line, role=status aria-live=polite,
    driven by `uaBusy`/`uaProgress` on projectTasks.
- No Terrain-side detail/persona/filter UI — UA's dashboard owns all graph
  interaction (E10); Terrain only hosts the frame and lifecycle.

### i18n keys (both locales, en + zh-CN/graph.ts)

- `ua.title`, `ua.descLine1`, `ua.dataDir` (`.terrain/understand/`)
- `ua.installTitle`, `ua.installHint`, `ua.pluginMissingTitle`, `ua.pluginMissingHint`
- `ua.missingGraphTitle`, `ua.missingGraphHint`, `ua.busy`, `ua.viewerStartFail`, `ua.loadFailed`
- `graph.views.{graphify,understand}` (view pills)
- `ua.stats.{nodes,links,built}` — mirror of the graphify stats chips

## Setup/packaging (env-catalog)

### New catalog.json entries (naming follows tool-graphify)

```jsonc
// top-level "version": 2 → 3
{
  "id": "tool-understand",
  "kind": "tool",
  "label": "Understand-Anything Toolchain",
  "description": "代码语义知识图谱（LLM 辅助）— Claude Code 插件 + 纯 Node 查看服务器；输出归档 .terrain/understand/",
  "optional": true,
  "check": ["node", "--version"],
  "install_steps": [   // SEQUENCE, fail-fast (current apply.rs semantics)
    { "cmd": "sh", "args": ["-c", "test -d \"$HOME/.terrain/understand-anything/repo/.git\" || git clone --depth 1 https://github.com/Egonex-AI/Understand-Anything \"$HOME/.terrain/understand-anything/repo\""] },
    { "cmd": "sh", "args": ["-c", "cd \"$HOME/.terrain/understand-anything/repo/understand-anything-plugin\" && pnpm install --frozen-lockfile && pnpm --filter @understand-anything/core build"] }
  ],
  "depends_on": []
},
{
  "id": "skill-understand",
  "kind": "skill",
  "label": "Understand Skill",
  "description": "代码语义知识图谱：运行 UA /understand 管线，输出归档 .terrain/understand/，先读 index 与报告，变更后增量更新",
  "skill_dir": "understand-skill",
  "depends_on": ["tool-understand", "skill-terrain-knowledge"]
}
```

- `env-catalog/skills/understand-skill/SKILL.md` is **Terrain-authored**
  (clean-room): a thin wrapper in the style of graphify-skill — UA presence
  check (Claude Code plugin vs `~/.understand-anything-plugin`), how to run
  /understand, where outputs go (`.ua/` → archive into `.terrain/understand/`),
  the cost warning (E19), and the incremental hook flow. UA's own SKILL.md
  prose is NOT copied.
- The heavyweight UA skill stays Claude-Code-native (E16); non-CC platforms
  point at `install.sh <platform>` — Terrain does not re-implement the
  14-platform symlink table.
- No `bundled` binary for tool-understand (build-from-source, per E18).

### agent-tools.template.json additions

```jsonc
{
  // existing keys unchanged; add:
  "understand_viewer": {
    "cmd": "npx",
    "args": ["--yes", "understand-anything-viewer"],
    "note": "Serve .ua/ or legacy .understand-anything/ graph at 127.0.0.1 with ?token=; Node >= 18"
  },
  "understand_repo": "~/.terrain/understand-anything/repo",
  "usage_understand": "Prefer the desktop app's Understand view; CLI fallback: npx understand-anything-viewer <project-dir> --no-open"
}
```

### agents-md fragment edits (version bump: tools 4→5, skills 3→4)

- `agents-md/tools.fragment`: add row
  `| Understand (UA) | npx understand-anything-viewer（需 Node ≥ 18）| 无降级 |`
  and a usage row: 语义级问题（LLM 图）先读 `.terrain/understand/`；
  架构/符号问题仍先读 `.terrain/graphify-out/`（AST 图零成本）。
- `agents-md/skills.fragment`: add row `understand-skill` with the same scope
  note; load order stays knowledge → codegraph / repomix → rtk → understand
  (understand last: it is the most expensive step, E19).

## Memory-bank interconnection (per terrain:memory reader)

- `ensure_project_layout` (terrain-core `paths.rs` scaffolding list) gains an
  `understand/` dir under `.terrain/`.
- `search.rs DOC_SUBDIRS` gains `understand` so `terrain knowledge search/read`
  resolves docs there.
- `citations.rs`: `is_terrain_knowledge_markdown_path` accepts `understand/…`
  (otherwise refs classify as SourceCode and misroute).
- Agent context: extend the collector pattern (`assets/project_meta.rs`
  knowledge-dir injection) to read `.terrain/understand/*.md` markdown — UA's
  JSON graphs are NOT injected wholesale into prompts (token-heavy); only
  index/summary markdown is.
- `.terrain/understand/index.md`: written by the understand-skill at the end of
  each pipeline run — links to `knowledge-graph.json`, `meta.json`
  (gitCommitHash / analyzedAt), the top-level layers list, and cross-links to
  `../graphify-out/GRAPH_REPORT.md` (AST graph) and `../agent/context.md`.
  This is the first machine-written non-hand-maintained knowledge dir; it is
  regenerable.
- Git policy (`git_policy.rs` const bodies): UA outputs are regenerable machine
  artifacts → GITIGNORE_BODY gains `.terrain/understand/**` (all of it local
  derivation; graphify-out's untracked-noise lesson applied). If the `.ua/`
  fallback applies (UNKNOWN-U1), a `gitignore`-kind integration adds `/.ua/`
  to the repo root .gitignore.
- Registration: projects surface only after a parent scan (registry requires
  `.terrain/index.md`); UA integration does not auto-register new projects.

## UNKNOWN / out of scope for v1

- **U1 (resolved v1)** — UA data-dir relocation: no env override exists for
  resolveUaDir (verified: viewer.mjs hardcodes `.ua/` + `.understand-anything/`
  only; the archive step is DROPPED — archiving breaks incremental updates and
  the viewer). v1: `.ua/` canonical; `.terrain/understand/` hosts the
  agent-written index.md only (memory-bank markdown; regenerable raw JSON stays
  in `.ua/`, gitignored via the gitignore-kind integration). Status probes the
  terrain dir last (status-only in v1).
- **U2** — Single-file self-contained UA dashboard build (srcdoc compat): no
  reader evidence of a vite option; deferred and not required by R5.
- **U3** — Viewer npm availability: consumption is `npx` of a GitHub-release
  tarball, not an npm-registry package; exact tarball URL and offline behavior
  need verification at packaging time.
- **U4** — Cross-platform viewer spawn (Windows `install.ps1` +
  `understand-anything-viewer.cmd` shell resolution) — v1 targets macOS first.
- **U5** — Figma skill (needs `FIGMA_TOKEN`) — out of scope; Terrain's env
  integration does not provision it.
- **U6** — UA subdomain graphs (frontend-/backend-/design-merged) surfaced
  beyond UA's own dashboard views — v1 shows UA's UI only.
- **U7** — tree-sitter grammar coverage vs Terrain's target languages (dart /
  swift are separate wasm workspace packages) — unsupported languages get
  metrics-only fidelity (documented to users, not hidden).
- **U8** — terrain-cli command parity (`terrain tools understand status/view`)
  for ACP use — deferred so panel and CLI do not diverge during v1 bring-up.

## v1 status (bring-up record)

Implemented against this spec on 2026-10-31:

- **Setup**: catalog v3 (+ `tool-understand`, `skill-understand`), new
  `env-catalog/skills/understand-skill/SKILL.md`, agent-tools.template.json
  `usage.understand` entry, fragments tools v5 / skills v4, GIT_POLICY_VERSION 3
  (understand/ gitignored; raw `.ua/` gitignored at repo root via a
  gitignore-kind integration).
- **Backend**: `ua_status_cmd` + `src-tauri commands/ua.rs (`ua_viewer_cmd`): spawn
  `node viewer.mjs <repo> --no-open --port=<ephemeral>`, parse `Dashboard URL:`
  banner, stale-registration replacement, `ua-viewer-ready` event, UaViewerMap
  in AppState; ua.rs driver in terrain-dashboard (status + resolver + tests).
  crates/terrain-core memory-bank: layout/search/citations/git_policy/
  agent-context collector now include understand/.
- **Frontend**: GraphifyPanel view pills (Graphify | Understand) — UA states:
  toolchain card, iframe (http src + allow-same-origin, token URL; no srcdoc per
  E14), start-viewer card, no-graph empty state; chips row reused
  (staleness/nodes/links/analyzed). i18n en + zh-CN.
- **Not implemented (agent-driven by design)**: run_ua_cmd "analyze" streaming
  (E20: no headless UA pipeline); the Understand empty-state points at the
  understand-skill flow; Start viewer serves existing .ua/ data.
