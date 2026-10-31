---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## Project Overview
Terrain is an **AI-friendly engineering-environment manager for coding agents** (author: Sopaco; v0.9.8; source packing via repomix-rs). Claim: generate docs from code, keep them in sync, make agents ready — knowledge is the map, tools are the paths, workflows are the signposts. From a Git repo it produces three assets: `human/` Litho narrative docs, `agent/context.md` structured agent context, and `agent/repomix.md` packed source index; plus agent contracts (Skills, `AGENTS.md`, JSON `terrain tools` API) and a toolchain (`codegraph`, `rtk`, `terrain`) deployed to `~/.terrain/bin`. Discipline: offline capabilities (scan, pack, retrieval, freshness, env integration) are LLM-free Rust; only generative tasks (Litho docs, agent context, DeepWiki Q&A, SDD) run in `terrain-agent` behind pluggable LLM/ACP backends — reviewable, degradable, never overwriting good assets. Consumers: desktop users, external agents (CLI/ACP), fleet operators (dashboard).
## Architecture
```
┌─────────────────────────────────────────────────────────────┐
│ Svelte 5 frontend (src/)      Tauri 2 shell (src-tauri/)    │
│  Overview · Wiki · Ask · SDD · Env · Graph · Git · Fleet     │
│  Usage · Tray                                                │
└──────────────────┬──────────────────────────────────────────┘
                   │ invoke + streaming events (ts-rs; Rust is truth)
┌──────────────────▼──────────────────────────────────────────┐
│ src-tauri commands (assets · env · git · dashboard ·         │
│  graphify · usage · workflows)                               │
│ terrain-cli (`terrain ask/tools/dashboard`) for agents/ACP   │
└──────────────────┬──────────────────────────────────────────┘
┌──────────────────▼──────────────────────────────────────────┐
│ terrain-core: assets · freshness · ingest · registry ·        │
│  gitops · vault · usage · sessions · search · schema (ts-rs)  │
├─────────────────────────────────────────────────────────────┤
│ terrain-agent (LLM chat/native+ACP, workflows) ·             │
│  terrain-dashboard (fleet + graphify wrapper) · ts-export    │
└──────────────────┬──────────────────────

…
## Module Map
| Module | Responsibility | Primary paths |
|---|---|---|
| terrain-core | Domain core: scan/pack, asset generation, three-tier retrieval, freshness, registry, `gitops`, `vault`, IPC types | `crates/terrain-core/src/` |
| assets/ | repomix pack, layered agent context, Litho/SDD/Ask assets, incremental refresh, env integration | `crates/terrain-core/src/assets/` |
| freshness/ | Git + CodeGraph drift drivers, ledger, drift factors, scoring + trust thresholds (`codegraph_drift`) | `crates/terrain-core/src/freshness/` |
| gitops.rs | Git worktree ops (status porcelain v2, branches, stage/commit/diff) behind the Git panel | `crates/terrain-core/src/gitops.rs` |
| vault.rs | Obsidian persistent-memory installer (autosave scripts, Claude SessionEnd hook, sync cron, per-project graph links) | `crates/terrain-core/src/vault.rs` |
| usage.rs | Local usage/cost probing; cccost preferred, ccusage fallback | `crates/terrain-core/src/usage.rs` |
| terrain-agent | LLM runtime: chat (native/ACP), model catalog & provider resolution (`LlmProvider`), workflows (ask/init/sdd/quick_refresh), tool schemas | `crates/terrain-agent/src/` |
| terrain-dashbo

…
## Core Flows
**1. Scan/pack → knowledge assets**
1. `initialize_project` registers the repo in `~/.terrain/registry.json` (path pointer only).
2. `ProjectScanner::scan_repo` produces `index.md` plus structured `modules/`, `interfaces/`, `routes/`, `events/` docs; optional OpenAPI import.
3. repomix-rs packs sources → `.terrain/agent/repomix.md`.
4. LLM generates `agent/context.md` (macro/meso layers); the optional Litho four-phase run produces `human/` docs; the freshness ledger records per-asset baseline commits for later drift scoring.

**2. Ask / DeepWiki three-tier retrieval**
Macro (`context.md` overview preloaded) → meso (`read_agent_context` sections) → micro (grep + ranged slices over the repomix pack), plus structured `modules/`/`interfaces/`/`routes/` hits. Freshness scoring plus the git-based CodeGraph drift cross-check yield a trust block telling agents what to trust and when to recompute (`terrain tools freshness`, `tools codegraph-drift`). On conflict precedence is: repomix sources > codegraph > `agent/context.md` > `human/`.

**3. Generative run pipeline (Litho / SDD / Ask / context refresh)**
Phases are planned in `crates/terrain-

…
## Tech Stack
- **Rust**: workspace v0.9.8 (terrain-core, terrain-agent, terrain-cli, terrain-dashboard, terrain-ts-export, src-tauri), edition 2024, rust-version 1.94
- **Desktop shell**: Tauri 2 (capabilities ACL, tray + Usage window, plugin-dialog/shell)
- **Frontend**: Svelte 5 runes + Vite + Tailwind 4; marked/mermaid/highlight.js; i18n en/zh-CN
- **IPC types**: ts-rs + schemars → generated TS bindings consumed by `src/lib/`
- **LLM**: OpenAI-compatible (`chat/completions` or `responses` modes), Ollama local/cloud, per-provider profiles; native models for lightweight phases
- **Toolchain**: repomix-rs, graphify (tree-sitter AST), CodeGraph, rtk; npm per-platform bundles → `~/.terrain/bin` (vendored bundles excluded from the agent pack)
- **Persistence**: `~/.terrain/` (registry, sessions, dashboard cache), per-repo `.terrain/` (repomix, graphify-out, knowledge), Obsidian `~/vault`
## System Boundaries
| Boundary | Direction / notes |
|---|---|
| Tauri IPC: Rust commands ↔ Svelte via `invoke` + streaming events; Rust types are the single truth (ts-rs) | internal |
| Agent surface: `terrain tools` JSON API + ACP subprocess — external coding agents consume knowledge without the desktop app | internal, agent-facing |
| LLM providers: OpenAI-compatible / Ollama local + cloud via profile catalog; only this boundary crosses the network for generation | outbound |
| Git operations: `gitops` shells out to git (porcelain v2, blocking tasks on a runtime); reachable only via explicit Git-panel / `git_*` commands — no background repo mutation | outbound, user-triggered |
| Code intelligence: graphify (local AST, no API cost) writes `.terrain/graphify-out/`; CodeGraph index staleness is cross-checked by terrain freshness before symbol queries are trusted | outbound local |
| Usage/cost: cccost (claude-code-cli-cost-tracker) preferred local source, ccusage fallback; strictly read-only probing | outbound local |
| Persistent memory: `~/vault` Obsidian vault + `~/scripts` autosave + Claude SessionEnd hook + nightly cron; machine-level only — never

…
## Code Map Index
| Concept | Location | Notes |
|---|---|---|
| Asset generation pipeline | `crates/terrain-core/src/assets/mod.rs` | repomix/context/litho/sdd/ask/env aggregation |
| repomix pack | `crates/terrain-core/src/assets/repomix.rs` | `pack_agent_assets`; vendored npm bundles excluded |
| Context layering | `crates/terrain-core/src/assets/context_layers.rs`, `agent_context.rs` | macro/meso split, size caps |
| Litho four-phase docs | `crates/terrain-core/src/prompts/`, `crates/terrain-agent/src/litho_native.rs` | preprocessing → research → composition → output |
| Model catalog / providers | `crates/terrain-agent/src/model.rs` | `LlmProvider`, `parse_provider`, `list_provider_models` |
| Freshness scoring & trust | `crates/terrain-core/src/freshness/scoring.rs`, `compute.rs`, `ledger.rs` | drift factors, thresholds, trust block |
| CodeGraph drift cross-check | `crates/terrain-core/src/freshness/codegraph.rs` | git-based staleness report (`codegraph_drift`) |
| Git worktree ops | `crates/terrain-core/src/gitops.rs`, `src-tauri/src/commands/git.rs` | Git panel commands |
| Vault installer | `crates/terrain-core/src/vault.rs` | `vault_root` (

…
## Project Overview
Terrain is an **AI-friendly engineering-environment manager for coding agents** (author: Sopaco; v0.9.8; source packing via repomix-rs). Claim: generate docs from code, keep them in sync, make agents ready — knowledge is the map, tools are the paths, workflows are the signposts. From a Git repo it produces three assets: `human/` Litho narrative docs, `agent/context.md` structured agent context, and `agent/repomix.md` packed source index; plus agent contracts (Skills, `AGENTS.md`, JSON `terrain tools` API) and a toolchain (`codegraph`, `rtk`, `terrain`) deployed to `~/.terrain/bin`. Discipline: offline capabilities (scan, pack, retrieval, freshness, env integration) are LLM-free Rust; only generative tasks (Litho docs, agent context, DeepWiki Q&A, SDD) run in `terrain-agent` behind pluggable LLM/ACP backends — reviewable, degradable, never overwriting good assets. Consumers: desktop users, external agents (CLI/ACP), fleet operators (dashboard).
## Architecture
```
┌─────────────────────────────────────────────────────────────┐
│ Svelte 5 frontend (src/)      Tauri 2 shell (src-tauri/)    │
│  Overview · Wiki · Ask · SDD · Env · Graph · Git · Fleet     │
│  Usage · Tray                                                │
└──────────────────┬──────────────────────────────────────────┘
                   │ invoke + streaming events (ts-rs; Rust is truth)
┌──────────────────▼──────────────────────────────────────────┐
│ src-tauri commands (assets · env · git · dashboard ·         │
│  graphify · usage · workflows)                               │
│ terrain-cli (`terrain ask/tools/dashboard`) for agents/ACP   │
└──────────────────┬──────────────────────────────────────────┘
┌──────────────────▼──────────────────────────────────────────┐
│ terrain-core: assets · freshness · ingest · registry ·        │
│  gitops · vault · usage · sessions · search · schema (ts-rs)  │
├─────────────────────────────────────────────────────────────┤
│ terrain-agent (LLM chat/native+ACP, workflows) ·             │
│  terrain-dashboard (fleet + graphify wrapper) · ts-export    │
└──────────────────┬──────────────────────

…
## Module Map
| Module | Responsibility | Primary paths |
|---|---|---|
| terrain-core | Domain core: scan/pack, asset generation, three-tier retrieval, freshness, registry, `gitops`, `vault`, IPC types | `crates/terrain-core/src/` |
| assets/ | repomix pack, layered agent context, Litho/SDD/Ask assets, incremental refresh, env integration | `crates/terrain-core/src/assets/` |
| freshness/ | Git + CodeGraph drift drivers, ledger, drift factors, scoring + trust thresholds (`codegraph_drift`) | `crates/terrain-core/src/freshness/` |
| gitops.rs | Git worktree ops (status porcelain v2, branches, stage/commit/diff) behind the Git panel | `crates/terrain-core/src/gitops.rs` |
| vault.rs | Obsidian persistent-memory installer (autosave scripts, Claude SessionEnd hook, sync cron, per-project graph links) | `crates/terrain-core/src/vault.rs` |
| usage.rs | Local usage/cost probing; cccost preferred, ccusage fallback | `crates/terrain-core/src/usage.rs` |
| terrain-agent | LLM runtime: chat (native/ACP), model catalog & provider resolution (`LlmProvider`), workflows (ask/init/sdd/quick_refresh), tool schemas | `crates/terrain-agent/src/` |
| terrain-dashbo

…
## Core Flows
**1. Scan/pack → knowledge assets**
1. `initialize_project` registers the repo in `~/.terrain/registry.json` (path pointer only).
2. `ProjectScanner::scan_repo` produces `index.md` plus structured `modules/`, `interfaces/`, `routes/`, `events/` docs; optional OpenAPI import.
3. repomix-rs packs sources → `.terrain/agent/repomix.md`.
4. LLM generates `agent/context.md` (macro/meso layers); the optional Litho four-phase run produces `human/` docs; the freshness ledger records per-asset baseline commits for later drift scoring.

**2. Ask / DeepWiki three-tier retrieval**
Macro (`context.md` overview preloaded) → meso (`read_agent_context` sections) → micro (grep + ranged slices over the repomix pack), plus structured `modules/`/`interfaces/`/`routes/` hits. Freshness scoring plus the git-based CodeGraph drift cross-check yield a trust block telling agents what to trust and when to recompute (`terrain tools freshness`, `tools codegraph-drift`). On conflict precedence is: repomix sources > codegraph > `agent/context.md` > `human/`.

**3. Generative run pipeline (Litho / SDD / Ask / context refresh)**
Phases are planned in `crates/terrain-

…
## Tech Stack
- **Rust**: workspace v0.9.8 (terrain-core, terrain-agent, terrain-cli, terrain-dashboard, terrain-ts-export, src-tauri), edition 2024, rust-version 1.94
- **Desktop shell**: Tauri 2 (capabilities ACL, tray + Usage window, plugin-dialog/shell)
- **Frontend**: Svelte 5 runes + Vite + Tailwind 4; marked/mermaid/highlight.js; i18n en/zh-CN
- **IPC types**: ts-rs + schemars → generated TS bindings consumed by `src/lib/`
- **LLM**: OpenAI-compatible (`chat/completions` or `responses` modes), Ollama local/cloud, per-provider profiles; native models for lightweight phases
- **Toolchain**: repomix-rs, graphify (tree-sitter AST), CodeGraph, rtk; npm per-platform bundles → `~/.terrain/bin` (vendored bundles excluded from the agent pack)
- **Persistence**: `~/.terrain/` (registry, sessions, dashboard cache), per-repo `.terrain/` (repomix, graphify-out, knowledge), Obsidian `~/vault`
## System Boundaries
| Boundary | Direction / notes |
|---|---|
| Tauri IPC: Rust commands ↔ Svelte via `invoke` + streaming events; Rust types are the single truth (ts-rs) | internal |
| Agent surface: `terrain tools` JSON API + ACP subprocess — external coding agents consume knowledge without the desktop app | internal, agent-facing |
| LLM providers: OpenAI-compatible / Ollama local + cloud via profile catalog; only this boundary crosses the network for generation | outbound |
| Git operations: `gitops` shells out to git (porcelain v2, blocking tasks on a runtime); reachable only via explicit Git-panel / `git_*` commands — no background repo mutation | outbound, user-triggered |
| Code intelligence: graphify (local AST, no API cost) writes `.terrain/graphify-out/`; CodeGraph index staleness is cross-checked by terrain freshness before symbol queries are trusted | outbound local |
| Usage/cost: cccost (claude-code-cli-cost-tracker) preferred local source, ccusage fallback; strictly read-only probing | outbound local |
| Persistent memory: `~/vault` Obsidian vault + `~/scripts` autosave + Claude SessionEnd hook + nightly cron; machine-level only — never

…
## Code Map Index
| Concept | Location | Notes |
|---|---|---|
| Asset generation pipeline | `crates/terrain-core/src/assets/mod.rs` | repomix/context/litho/sdd/ask/env aggregation |
| repomix pack | `crates/terrain-core/src/assets/repomix.rs` | `pack_agent_assets`; vendored npm bundles excluded |
| Context layering | `crates/terrain-core/src/assets/context_layers.rs`, `agent_context.rs` | macro/meso split, size caps |
| Litho four-phase docs | `crates/terrain-core/src/prompts/`, `crates/terrain-agent/src/litho_native.rs` | preprocessing → research → composition → output |
| Model catalog / providers | `crates/terrain-agent/src/model.rs` | `LlmProvider`, `parse_provider`, `list_provider_models` |
| Freshness scoring & trust | `crates/terrain-core/src/freshness/scoring.rs`, `compute.rs`, `ledger.rs` | drift factors, thresholds, trust block |
| CodeGraph drift cross-check | `crates/terrain-core/src/freshness/codegraph.rs` | git-based staleness report (`codegraph_drift`) |
| Git worktree ops | `crates/terrain-core/src/gitops.rs`, `src-tauri/src/commands/git.rs` | Git panel commands |
| Vault installer | `crates/terrain-core/src/vault.rs` | `vault_root` (

…