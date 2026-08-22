---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## Project Overview

Terrain is an **engineering environment management platform** for the AI coding-assistant era (Sopaco open source, includes repomix-rs). Core idea: "Terrain prepares the ground so agents don't have to guess where to stand" — register a Git repo and it automatically scans code, packs sources (repomix), generates Agent context, C4 architecture docs (Litho), dual-track knowledge assets (human `human/` + Agent `agent/`), and exposes Ask Q&A and a four-phase SDD workflow to external Coding Agents. Knowledge lives in-repo under `.terrain/` and flows with Git branches ("knowledge follows code"). Consumers: desktop app (Tauri+Svelte), CLI (`terrain`/`terrain tools`), external Coding Agents (ACP subprocess). Key constraints: Rust is the sole IPC source of truth (ts-rs → TS), context hard cap 16 KiB, non-deterministic generated assets must not auto-merge, grep the pack rather than read all code.

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│ Frontend Svelte 5 (src/)              Tauri 2 shell (src-tauri/) │
│  Ask/DeepWiki · SDD · Litho · Env · Projects · Usage · Tray   │
└───────────────┬──────────────────────────────────────────────┘
                │ invoke + streaming events (ts-rs types, Rust source of truth)
┌───────────────▼──────────────────────────────────────────────┐
│ terrain-core  domain core (pure logic, no LLM execution)        │
│  assets/ generation · query/search 3-layer retrieval · freshness/ │
│  ingest/scan · registry · sessions · ipc+schema types           │
└───────────────┬──────────────────────────────────────────────┘
┌───────────────▼──────────────────────────────────────────────┐
│ terrain-agent execution layer                                   │
│  ChatEngine(Native ADK / ACP dual backend) · tools · context gen │
│  workflows: Ask / Init / SDD / QuickRefresh                      │
└───────────────┬──────────────────────────────────────────────┘
                │ adk-model(openai chat/responses, ollama) · agent-client-protocol(ACP)
┌───────────────▼───────────────┬───────────────┬──────────────┐
│ repomix-core pack · codegraph   │ LLM Providers │ External Agent │
│ (SQLite) · rtk                  │ (OpenAI/Ollama)│ opencode       │
└───────────────────────────────┴───────────────┴──────────────┘
```

- **Layers**: UI (Svelte) → IPC shell (src-tauri commands) → core (terrain-core, domain logic/assets, no execution) → execution (terrain-agent, LLM/ACP/tools/workflows).
- **Dependency direction**: terrain-agent → terrain-core; src-tauri, terrain-cli → both; `[patch.crates-io]` locally replaces `agent-client-protocol-tokio`.
- **Three entry points, one core**: desktop app, CLI (`terrain-cli` direct; `tools` subcommand is Ask's ACP knowledge layer), npm packages (`cli`/`rtk` binary shims).
- **Build-time type flow**: ts-rs annotations (`ts-export` feature) → `terrain-ts-export` binary → `src/lib/generated/` (`bun run gen:types`).

## Module Map

| Module | Responsibility | Primary paths |
|------|------|----------|
| terrain-core | Domain core: asset generation, 3-layer retrieval, freshness, ingest, registry, IPC types | `crates/terrain-core/src/` |
| ├ assets/ | repomix pack, agent context, litho/sdd/ask assets, incremental refresh, context layers, env integration | `crates/terrain-core/src/assets/` |
| ├ freshness · ingest · schema · sessions | git+codegraph freshness scoring, ProjectScanner/OpenAPI, ts-rs structs, session persistence | `crates/terrain-core/src/{freshness,ingest,schema,sessions,ipc}/` |
| terrain-agent | Execution: ChatEngine, tool schema/registry, context generation, ACP settings, SDD/Litho driver | `crates/terrain-agent/src/` |
| ├ chat/ | ChatEngine dual backend (native.rs=ADK Runner / acp.rs), prompt, tracker, types | `crates/terrain-agent/src/chat/` |
| └ workflows/ · runtime | Ask/Init/SDD/QuickRefresh orchestration; Runtime engine cache & ModelConfig | `crates/terrain-agent/src/workflows/`、`runtime.rs`、`acp.rs`、`tools.rs` |
| terrain-cli | CLI entry: list/scan/init/refresh/search/read + project/settings/ask/sdd/usage/source/tools/assets/env | `crates/terrain-cli/src/` |
| src-tauri | Desktop shell: commands (project/sessions/workflows/knowledge/env/usage/assets/settings), tray, bundled tools, preset skills | `src-tauri/src/` |
| src/ (Svelte 5) | UI: Ask/DeepWiki, SDD, Litho, env panel, usage, project overview, knowledge tree | `src/lib/`（components/、stores/、api.ts） |
| terrain-ts-export | Build-time ts-rs export binary (root types → `src/lib/generated/`) | `crates/terrain-ts-export/src/main.rs` |
| env-catalog + preset_skills | Env injection assets: skills, agents-md fragments, tool catalog; built-in skills (ask/architecture/litho/sdd) | `env-catalog/`、`preset_skills/` |
| npm/ + packages/ + ACP patch | Cross-platform CLI/RTK binary shims & release; local ACP tokio patch dependency | `npm/packages/`、`packages/`、`crates/agent-client-protocol-tokio-patched/` |

## Core Flows

**1. Project registration → knowledge asset generation**
1. `initialize_project` registers the repo in `~/.terrain/registry.json` (path only).
2. `ProjectScanner` collects Git metadata and imports OpenAPI on demand (`scan_project`).
3. repomix-core packs sources → `.terrain/agent/repomix.md` (`pack_agent_assets`).
4. LLM generates `agent/context.md` (this document); optional Litho four-phase generation of `human/` C4 docs (preprocess→research→compose→output; research artifacts persist in `.litho-agent/` for resume).
5. freshness baseline ledger; subsequent git/codegraph cross-check for drift. Incremental context refresh and sync checks use `agent_context_recorded_baseline_head` (`context-meta.json` only) — a repomix repack advancing pack meta does not mark stale `context.md` as synced.

**2. Ask knowledge Q&A (DeepWiki, 3-layer retrieval + dual backend)**
1. Macro: preload `agent/context.md` overview/architecture/module map.
2. Meso: on demand `read_agent_context(section=…)` or search `human/`, `knowledge/` docs.
3. Micro: `grep_agent_pack` → `read_agent_pack_file` for source slices.
4. `ChatEngine` execution: Native backend (ADK Runner; OpenAI-compatible via `chat/completions` or `responses` per `OpenAiApiMode`, or Ollama) or ACP subprocess (opencode; `AcpSettings` gates `execution_uses_native_llm` / `execution_uses_acp` / `execution_pure_acp` / `AcpNative`); falls back to `fallback_search_reply` when LLM unavailable.
5. Stream thinking/tool_calls/phase/usage events (`AskStreamEvent`) + source citations; optional session persistence.

**3. SDD four-phase development**
1. Requirements → `1.requirements.md`.
2. TechDesign → `2.tech-design.md`.
3. Codegen → `3.implementation.md` + repo changes (delegated to ACP Agent).
4. CodeReview. Lightweight doc phases use Native LLM; code phase uses ACP (`run_sdd_phase` dispatches per phase); each phase produces reviewable Markdown.

**4. Environment integration (Env)**
1. Probe Skills / CLI tools / AGENTS.md status (`EnvStatus`).
2. Plan diff → `EnvPlan`/`EnvPlanStep`.
3. Apply: deploy terrain-knowledge/repomix/codegraph/rtk skills, bundled tools, `AGENTS.md` fragments (`plan_env_integration`/`apply_env_integration`, `deploy_agent_toolchain`).

## Tech Stack

- **Rust**: workspace (terrain-core/terrain-agent/terrain-cli/terrain-ts-export/src-tauri), edition 2024, rust-version 1.94.
- **Desktop shell**: Tauri 2 (capabilities ACL, plugin-dialog/shell, tray + Usage window).
- **Frontend**: Svelte 5 (runes) + Vite 8 + Tailwind 4 + marked/mermaid/highlight.js, @lucide/svelte.
- **Frontend bootstrap optimization**: `appBootstrap.ts` singleton cache deduplicates `bootstrapApp` IPC (shared once across main + usage windows); Vite `modulePreload` filters mermaid chunk to reduce first-screen preload size.
- **Frontend error display**: `errorFormat.ts` (`formatErrorDisplay` summary+detail) + `ErrorNotice.svelte`; `status.svelte.ts` exposes `setErrorStatus`/`setLocalizedErrorStatus`; `StatusBanner` supports expandable detail + copy; Ask errors carry `isError`/`errorDetail` on `ChatMessage`.
- **Ask share image**: off-screen mount of real `AskShareCard` (reuses MarkdownViewer/markdown.css/mermaid) → long-text pagination → native canvas rasterization to PNG; clipboard via `copy_image_to_clipboard`, save via `save_png_files`.
- **IPC types**: ts-rs 10 + schemars; `bun run gen:types` → `src/lib/generated/` (includes `OpenAiApiMode`).
- **Agent runtime**: ADK Rust 1.0 family (adk-core/agent/runner/session/tool/model `{openai,ollama}`/acp) + agent-client-protocol 0.11.1 (ACP subprocess), local `[patch]` tokio layer; Native OpenAI routing via `OpenAiApiMode` (`chat_completions` → `OpenAIClient`, `responses` → `OpenAIResponsesClient` with open-responses mode for third-party bases); override via `TERRAIN_OPENAI_API_MODE`.
- **Source index**: repomix-core 2.0 (repomix-rs Rust port) packs `agent/repomix.md`; CodeGraph (SQLite symbol graph) for drift cross-check; RTK compresses shell output.
- **Storage**: `.terrain/` (versioned knowledge), `~/.terrain/registry.json` (project pointers), `.codegraph/` (local index).
- **Distribution**: npm packages (`cli`/`rtk` + darwin-arm64/win32-x64 shims), `scripts/cross-windows-terrain.sh` cross-compile, Tauri bundle.
- **Release build**: workspace-level `lto = "thin"` + `strip = true` + `codegen-units = 1` for smaller artifacts and link-time tradeoff.
- **Base libs**: tokio, serde/serde_json, anyhow/thiserror, tracing, chrono, walkdir/ignore, futures, slug, dotenvy.

## System Boundaries

| Boundary | Description | Direction |
|------|------|------|
| Tauri IPC | Rust commands ↔ Svelte (`invoke` + streaming events); Rust types are source of truth | Internal |
| LLM Providers | OpenAI-compatible (`chat/completions` or `responses` per `OpenAiApiMode`) / Ollama (adk-model); Native for lightweight phases (Ask summary, SDD docs) | Out |
| ACP subprocess | External Coding Agent (opencode) via agent-client-protocol (SDD codegen/Litho/Ask); `acp_config_json` injects config & env; can spawn arbitrary command → trust boundary, gated by `AcpSettings` | Out |
| Local registry | `~/.terrain/registry.json` stores project paths only, no knowledge body | Local |
| Knowledge filesystem | `.terrain/agent/` (generated), `human/` (generated), `knowledge/` (manual), `.litho-agent/` (research artifacts), `repomix.md` (local index) | Local |
| External code | Read-only scan/pack (git metadata, OpenAPI import, repomix); does not write target repo (except SDD Codegen) | Out |
| Tool binaries | CodeGraph / RTK / terrain CLI shipped with project (`packages/`, `~/.terrain/bin/`, npm shims) | Out |
| Git | Ingest, freshness baseline compare (baseline HEAD, ledger), `.gitattributes` marks generated assets `-merge` | Out |

Trust boundaries: frontend does not trust Rust return values (Rust validates); ACP subprocess is external (spawn arbitrary command, requires authorized config); generated assets are non-deterministic → conflicts must not be hand-merged; keep either version and rerun scan to regenerate. IPC `Option<T>` → `T | null`; frontend null-checks per this contract; `read-pack-file`/`grep-pack` are the Agent's only source-code entry points.

## Code Map Index

| Concept | Location | Notes |
|------|------|------|
| Asset generation pipeline | `crates/terrain-core/src/assets/mod.rs` | repomix/context/litho/sdd/ask/env aggregation |
| repomix pack | `crates/terrain-core/src/assets/repomix.rs` | `pack_agent_assets`, pack freshness |
| Context layers / generation | `crates/terrain-core/src/assets/context_layers.rs`、`agent_context.rs` | macro/meso/on-demand slices; `agent_context_recorded_baseline_head` for refresh decisions; `agent_context_synced_with_head` ignores pack-only baseline |
| Incremental refresh | `crates/terrain-core/src/assets/incremental.rs`、`crates/terrain-agent/src/workflows/quick_refresh.rs` | |
| Litho generation | `crates/terrain-core/src/assets/litho.rs` + `crates/terrain-agent/src/litho.rs` | four phases, `.litho-agent/` resume |
| SDD workflow | `crates/terrain-agent/src/workflows/sdd.rs`、`crates/terrain-agent/src/sdd.rs` | phase dispatch LLM/ACP |
| Ask retrieval | `crates/terrain-core/src/assets/ask.rs`、`crates/terrain-agent/src/workflows/ask.rs` | 3-layer retrieval + fallback |
| ChatEngine dual backend | `crates/terrain-agent/src/chat/mod.rs`、`native.rs`、`acp.rs` | ADK Runner / ACP |
| Native LLM / OpenAI API mode | `crates/terrain-agent/src/model.rs`、`crates/terrain-core/src/settings.rs` | `OpenAiApiMode` on `ProviderProfile`/`ModelConfig`; `build_openai_llm` branches chat vs responses |
| Knowledge search & doc read | `crates/terrain-core/src/search.rs` | `KnowledgeSearch` 3-layer search; `read_doc_at`/`read_doc_at_in_project` accept absolute path, knowledge-root relative path, **bare filename** (e.g. `core` → `modules/core.md`); `SearchHit` includes `rel_path` for `read_doc` |
| Tool schema / registry | `crates/terrain-agent/src/tools.rs`、`tool_schema.rs`、`compat_tool.rs`、`tool_session_cache.rs` | `read_doc` bare-filename resolution; `search_knowledge` returns `rel_path` |
| Runtime / engine cache | `crates/terrain-agent/src/runtime.rs`、`builder.rs` | ModelConfig + AcpSettings |
| Freshness | `crates/terrain-core/src/freshness/` | compute/scoring/git/codegraph/drift_factors/ledger |
| Ingest / registry | `crates/terrain-core/src/ingest/`（git/openapi）、`registry.rs`、`project.rs` | ProjectScanner/ScanReport |
| Env integration | `crates/terrain-core/src/integrations/`、`assets/env/`、`agent_tools_deploy.rs`、`bundled_tools.rs` | EnvPlan/Status, usage probe |
| IPC types | `crates/terrain-core/src/schema/`、`ipc/`、`crates/terrain-agent/src/chat/types.rs` | ts-export annotations; `OpenAiApiMode.ts` |
| Tauri command layer | `src-tauri/src/commands/` | project/sessions/workflows/knowledge/env/usage/assets |
| Frontend bootstrap | `src/lib/appBootstrap.ts`、`src/main.ts` | `loadAppBootstrap()` singleton dedupes `bootstrapApp` IPC; main + usage windows share first call; `applyLocale` exported from `i18n/index.ts` |
| Frontend doc path helpers | `src/lib/humanDocPath.ts` | `humanDocTreePath()` for human-knowledge tree display; `findHumanOverviewDoc()` locates Litho overview (`1.概述.md`/`1.Overview.md`) |
| Frontend error formatting | `src/lib/errorFormat.ts`、`components/ErrorNotice.svelte`、`stores/status.svelte.ts` | `formatErrorDisplay`; `setErrorStatus`/`setLocalizedErrorStatus`; used in App/DeepWiki/Settings/Usage panels |
| Frontend IPC wrapper | `src/lib/api.ts`、`types.ts`、`types.client.ts` | invoke + generated type entry; `ChatMessage.isError`/`errorDetail` |
| Ask share / long-image export | `src/lib/askShareImage.ts`、`components/AskShareCard.svelte`、`ShareImageButton.svelte`、`shareExport.ts`、`clipboard.ts`、`src-tauri/src/commands/settings.rs` | off-screen MarkdownViewer render, paginated PNG rasterization; copy/export via `copy_image_to_clipboard`/`save_png_files` |
| CLI + Ask tools | `crates/terrain-cli/src/cli.rs`、`commands/tools.rs` | `terrain tools` knowledge layer |
| ACP protocol patch | `crates/agent-client-protocol-tokio-patched/src/acp_agent.rs` | Cargo patch |