---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## Project Overview

Terrain is an **agent-first engineering environment platform** (Sopaco open source; repomix-rs powers source packing). Tagline: *"Terrain prepares the ground so agents don't have to guess where to stand."* Point it at a Git repository and it scans code, packs sources (repomix), generates agent context, C4 architecture docs (Litho), dual-track knowledge (`human/` + `agent/`), freshness tracking, and exposes Ask Q&A plus a four-phase SDD workflow to external Coding Agents. Knowledge lives in-repo under `.terrain/` and travels with Git branches. Consumers: Tauri desktop app (Svelte UI), CLI (`terrain` / `terrain tools`), and external agents via ACP subprocess. Constraints: Rust is IPC source of truth (ts-rs → TypeScript); `context.md` hard cap 16 KiB; generated assets are non-deterministic (regenerate, don't hand-merge); agents query the repomix pack, not the live filesystem.

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│ Svelte 5 frontend (src/)          Tauri 2 shell (src-tauri/) │
│  Ask/DeepWiki · SDD · Litho · Env · Projects · Usage · Tray│
└───────────────┬──────────────────────────────────────────────┘
                │ invoke + streaming events (ts-rs; Rust = truth)
┌───────────────▼──────────────────────────────────────────────┐
│ terrain-core — domain core (no LLM execution)                │
│  assets/ · search/query · freshness · ingest · registry      │
│  sessions · ipc+schema types · env integration               │
└───────────────┬──────────────────────────────────────────────┘
┌───────────────▼──────────────────────────────────────────────┐
│ terrain-agent — execution layer                              │
│  ChatEngine (Native ADK / ACP) · tools · context generation  │
│  workflows: Ask · Init · SDD · QuickRefresh                  │
└───────────────┬──────────────────────────────────────────────┘
                │ adk-model (OpenAI/Ollama) · agent-client-protocol
┌───────────────▼───────────────┬───────────────┬──────────────┐
│ repomix-core · CodeGraph (SQLite)│ LLM Providers │ ACP Agent   │
│ RTK · bundled CLI shims         │ OpenAI/Ollama │ (opencode)  │
└───────────────────────────────┴───────────────┴──────────────┘
```

| Layer | Role | Key paths |
|-------|------|-----------|
| UI | Panels, stores, i18n (en/zh-CN) | `src/`, `src/lib/api.ts` |
| IPC shell | Tauri commands, tray, bundled tools | `src-tauri/src/commands/` |
| Domain core | Asset generation, 3-layer retrieval, freshness, ingest | `crates/terrain-core/` |
| Execution | ChatEngine, workflows, Litho/SDD driver, tool registry | `crates/terrain-agent/` |
| Entry points | Desktop app, `terrain-cli`, npm shims (`cli`/`rtk`) | All share core + agent |

- **Dependency direction**: terrain-agent → terrain-core; src-tauri & terrain-cli → both; `[patch.crates-io]` replaces `agent-client-protocol-tokio` locally.
- **Type flow**: ts-rs (`ts-export` feature) → `terrain-ts-export` → `src/lib/generated/` via `bun run gen:types`.
- **Design principle**: separate knowledge logic (core) from LLM/ACP execution (agent).

## Module Map

| Module | Responsibility | Primary paths |
|--------|----------------|---------------|
| terrain-core | Domain core: asset generation, 3-layer retrieval, freshness, ingest, registry, IPC types | `crates/terrain-core/src/` |
| assets/ | repomix pack, agent context, Litho/SDD/Ask assets, incremental refresh, env integration | `crates/terrain-core/src/assets/` |
| freshness | Git + CodeGraph drift scoring, baseline ledger | `crates/terrain-core/src/freshness/` |
| ingest | Project scan, Git metadata, OpenAPI import | `crates/terrain-core/src/ingest/` |
| terrain-agent | ChatEngine, workflows, Litho/SDD driver, ACP/native backends, tool registry | `crates/terrain-agent/src/` |
| chat/ | Dual backend: Native ADK Runner + ACP subprocess | `crates/terrain-agent/src/chat/` |
| workflows/ | Ask, Init, SDD, QuickRefresh orchestration | `crates/terrain-agent/src/workflows/` |
| terrain-cli | Headless entry: scan, init, ask, tools, env, usage | `crates/terrain-cli/src/commands/` |
| src-tauri | Desktop shell: IPC commands, tray, preset skills, env catalog | `src-tauri/src/` |
| Frontend | Svelte 5 UI, IPC wrappers, stores, i18n | `src/lib/` |
| preset_skills | Bundled agent skills (Litho, SDD, Ask, arch, context) | `preset_skills/` |
| env-catalog | Agent toolchain catalog, AGENTS.md fragments, skill templates | `env-catalog/` |

## Core Flows

**1. Project registration → knowledge asset generation**
1. `initialize_project` registers repo in `~/.terrain/registry.json` (path pointer only).
2. `ProjectScanner` collects Git metadata; optional OpenAPI import (`scan_project`).
3. repomix-core packs sources → `.terrain/agent/repomix.md` (`pack_agent_assets`).
4. LLM generates `agent/context.md`; optional Litho four-phase run produces `human/` C4 docs (artifacts persist in `.litho-agent/` for resume).
5. Freshness baseline ledger written; subsequent git/codegraph cross-check for drift. Incremental context refresh uses `agent_context_recorded_baseline_head` (`context-meta.json`) — repomix repack alone does not mark `context.md` synced.

**2. Ask knowledge Q&A (DeepWiki, 3-layer retrieval + dual backend)**
1. Macro: preload `agent/context.md` overview/architecture/module map.
2. Meso: on demand `read_agent_context(section=…)` or search `human/`, `knowledge/` docs.
3. Micro: `grep_agent_pack` → `read_agent_pack_file` for source slices.
4. `ChatEngine` execution: Native (ADK Runner; OpenAI `chat/completions` or `responses` per `OpenAiApiMode`, or Ollama) or ACP subprocess (opencode); gated by `AcpSettings`; falls back to `fallback_search_reply` when LLM unavailable.
5. Stream thinking/tool_calls/phase/usage events (`AskStreamEvent`) + source citations; optional session persistence.

**3. SDD four-phase development**
1. Requirements → `1.requirements.md`.
2. TechDesign → `2.tech-design.md`.
3. Codegen → `3.implementation.md` + repo changes (delegated to ACP Agent).
4. CodeReview. Doc phases use Native LLM; code phase uses ACP (`run_sdd_phase` dispatches per phase).

**4. Environment integration (Env)**
1. Probe Skills / CLI tools / AGENTS.md status (`EnvStatus`).
2. Plan diff → `EnvPlan` / `EnvPlanStep`.
3. Apply: deploy terrain-knowledge/repomix/codegraph/rtk skills, bundled tools, `AGENTS.md` fragments.

## Tech Stack

- **Rust**: workspace (terrain-core, terrain-agent, terrain-cli, terrain-ts-export, src-tauri), edition 2024, rust-version 1.94
- **Desktop shell**: Tauri 2 (capabilities ACL, plugin-dialog/shell, tray + Usage window)
- **Frontend**: Svelte 5 (runes) + Vite 8 + Tailwind 4 + marked/mermaid/highlight.js
- **IPC types**: ts-rs 10 + schemars; `bun run gen:types` → `src/lib/generated/`
- **Agent runtime**: ADK Rust 1.0 (adk-core/agent/runner/session/tool/model) + agent-client-protocol 0.11.1 (ACP); local `[patch]` tokio layer; `OpenAiApiMode` routes chat vs responses API
- **Source index**: repomix-core 2.0 (repomix-rs) → `agent/repomix.md`; CodeGraph (SQLite symbol graph); RTK compresses shell output
- **Storage**: `.terrain/` (versioned knowledge), `~/.terrain/registry.json` (project pointers), `.codegraph/` (local index)
- **Distribution**: npm packages (`cli`/`rtk` + darwin-arm64/win32-x64 shims), Tauri bundle; release profile `lto=thin`, `strip=true`
- **Base libs**: tokio, serde/serde_json, anyhow/thiserror, tracing, chrono, walkdir/ignore, futures

## System Boundaries

| Boundary | Description | Direction |
|----------|-------------|-----------|
| Tauri IPC | Rust commands ↔ Svelte (`invoke` + streaming events); Rust types are source of truth | Internal |
| LLM Providers | OpenAI-compatible (`chat/completions` or `responses` per `OpenAiApiMode`) / Ollama; Native for lightweight phases | Out |
| ACP subprocess | External Coding Agent (opencode) via agent-client-protocol; `acp_config_json` injects config; can spawn arbitrary command → trust boundary, gated by `AcpSettings` | Out |
| Local registry | `~/.terrain/registry.json` stores project paths only, no knowledge body | Local |
| Knowledge filesystem | `.terrain/agent/` (generated), `human/` (generated), `knowledge/` (manual), `.litho-agent/` (research), `repomix.md` (local index) | Local |
| External code | Read-only scan/pack (git, OpenAPI, repomix); does not write target repo (except SDD Codegen) | Out |
| Tool binaries | CodeGraph / RTK / terrain CLI (`packages/`, `~/.terrain/bin/`, npm shims) | Out |
| Git | Ingest, freshness baseline, `.gitattributes` marks generated assets `-merge` | Out |

Trust: frontend null-checks IPC `Option<T>` → `T | null`; ACP subprocess is external; generated assets are non-deterministic — regenerate rather than hand-merge; `grep-pack`/`read-pack-file` are Agent's only source-code entry points (not live filesystem).

## Code Map Index

| Concept | Location | Notes |
|---------|----------|-------|
| Asset generation pipeline | `crates/terrain-core/src/assets/mod.rs` | repomix/context/litho/sdd/ask/env aggregation |
| repomix pack | `crates/terrain-core/src/assets/repomix.rs` | `pack_agent_assets`, pack freshness |
| Context layers / generation | `crates/terrain-core/src/assets/context_layers.rs`, `agent_context.rs` | macro/meso slices; baseline-head refresh |
| Context generator (agent) | `crates/terrain-agent/src/context_generator.rs`, `agent_context.rs` | LLM-driven context synthesis |
| Incremental refresh | `crates/terrain-core/src/assets/incremental.rs`, `crates/terrain-agent/src/workflows/quick_refresh.rs` | delta updates |
| Litho generation | `crates/terrain-core/src/assets/litho.rs`, `crates/terrain-agent/src/litho.rs` | four phases, `.litho-agent/` resume |
| SDD workflow | `crates/terrain-agent/src/workflows/sdd.rs`, `crates/terrain-agent/src/sdd.rs` | phase dispatch LLM/ACP |
| Ask retrieval | `crates/terrain-core/src/assets/ask.rs`, `crates/terrain-agent/src/workflows/ask.rs` | 3-layer retrieval + fallback |
| ChatEngine dual backend | `crates/terrain-agent/src/chat/mod.rs`, `native.rs`, `acp.rs` | ADK Runner / ACP |
| Knowledge search & doc read | `crates/terrain-core/src/search.rs` | `KnowledgeSearch`; `read_doc_at` |
| Freshness | `crates/terrain-core/src/freshness/` | compute/scoring/git/codegraph/ledger |
| Env integration | `crates/terrain-core/src/assets/env/`, `agent_tools_deploy.rs` | EnvPlan/Status, toolchain deploy |
| IPC types | `crates/terrain-core/src/schema/`, `ipc/` | ts-export annotations |
| Tauri command layer | `src-tauri/src/commands/` | project/sessions/workflows/knowledge/env/usage |
| Frontend IPC wrapper | `src/lib/api.ts`, `types.ts`, `appBootstrap.ts` | invoke + bootstrap singleton dedup |
| CLI + Ask tools | `crates/terrain-cli/src/commands/tools.rs` | `terrain tools` knowledge layer |
| ACP protocol patch | `crates/agent-client-protocol-tokio-patched/src/acp_agent.rs` | Cargo `[patch]` |