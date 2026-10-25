---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## Project Overview
`,
 `
## Architecture
`,
 `
## Module Map
`,
 `
## Core Flows
`,
 `
## Tech Stack
`,
 `
## System Boundaries
`,
 `
## Code Map Index
`.

Terrain 是一个**标准化、AI 友好的工程环境管理工具**：指向一个 Git 仓库，自动产出多层知识资产（人类可读的 `human/` C4 文档、Agent 消费的 `agent/context.md` 与源码索引 `agent/repomix.md`、业务术语 `knowledge/`，以及从 OpenAPI 规范离线抽取的 `interfaces/`、`routes/` 结构化条目），并一键部署 Agent 工具链（CodeGraph、RTK、预设 Skills、受管理的 `AGENTS.md` 片段）。隐喻：知识是地图，工具是道路，流程/约定是路标。提供桌面应用（Tauri GUI）与 CLI 双形态，外部编码 Agent（Claude Code、Codex、OpenCode、ACP）通过 `terrain tools` JSON 接口消费同一份知识。核心约束：scan/pack/search/freshness/env 全部离线纯 Rust，不依赖 LLM；知识资产随 Git 分支流转（`.terrain/` 入库）；采用增量更新 + 新鲜度评分追踪与代码的漂移；引文行号只在 pack 切片行号可信时可用，折叠/压缩视图不得按行号引用。

 preserve


 the

 diagram

 (ASCII),
 translated
 labels.


 Diagram

 content


 translation
 is
 allowed

 (
"diagram
 labels
 must
 all use

 this
 language
").


 I


 must
 keep
 the

 diagram's
 shape
.

Let me
 now


 also
 double

-check


 one


 thing
:

 the
 overview


 says


 "consum
ers... and external
 Agents
 connected
 via
 ACP
 subprocess" 
— unchanged
.


 Als

…
## Module
Map

 —

 table

 with

 rows
 (translate
 all

;
 terrain
-agent

 row
 gains 



"
model

catalog/provider

resolution

 (LlmProvider
)")
## Core
Flows
 — 4
 numbered
 flows
 with
 bold
 titles
.
## Tech
Stack


 — bullets
.
## System
Boundaries
 — table

 +
 trust
 rules
 paragraph
.
## Code
Map
 Index
 — table
.


 Now

 let


 me


 count


 characters


 roughly
.


 I'll


 write


 it,
 then
 estimate

:
 the
 
ASCII
 
diagram

 ~1700
 chars
.
 Tables
: layer
 
table

 ~450
, module
 map
 
~1200
,
 boundaries
 
~1500
,
 code
 map
 
~1700
.
 Flows


 ~2800
.
 Overview
 ~1100
.
 Tech
 stack

 ~1100
 
. Trust
 paragraph

 ~900
.
 Total


 ~
13
,
5
00
.
 Tight
 but
 OK
.


 I


…
## Project Overview
Terrain is an **engineering-environment platform for coding agents** (Sopaco open-source; source packing driven by repomix-rs). Tagline: *"Terrain lays the ground so agents never have to guess where to stand."* Pointed at a Git repo, it scans code, packs sources (repomix), generates Agent context, C4 architecture docs (Litho), dual-track knowledge (`human/` + `agent/`), and knowledge-freshn

…
## Architecture
```
┌───────────────────────────────────────────────────────────────┐
│ Svelte 5 frontend (src/)        Tauri 2 shell (src-tauri/)    │
│  Ask/DeepWiki · SDD · Litho · Env · Projects · Usage · Tray   │
└───────────────┬───────────────────────────────────────────────┘
                │ invoke + streaming events (ts-rs; Rust is IPC truth)
┌───────────────▼─────────────────────────────────────

…
## Module Map
| Module | Responsibility | Main paths |
|--------|----------------|------------|
| terrain-core | Domain core: asset generation, three-tier retrieval, freshness, ingest, registry, IPC types | `crates/terrain-core/src/` |
| assets/ | repomix pack, agent context, Litho/SDD/Ask assets, incremental refresh, env integration | `crates/terrain-core/src/assets/` |
| freshness | Git + CodeGraph dri

…
## Core Flows
**1. Project registration → knowledge asset generation**
1. `initialize_project` registers the repo into `~/.terrain/registry.json` (path pointer only).
2. `ProjectScanner` collects Git metadata; optional OpenAPI import (`scan_project`).
3. repomix-core packs sources → `.terrain/agent/repomix.md` (`pack_agent_assets`).
4. LLM generates `agent/context.md`; the optional Litho four-phase run p

…
## Tech Stack
- **Rust**: workspace (terrain-core, terrain-agent, terrain-cli, terrain-ts-export, src-tauri), edition 2024, rust-version 1.94
- **Desktop shell**: Tauri 2 (capabilities ACL, plugin-dialog/shell, tray + Usage window)
- **Frontend**: Svelte 5 (runes) + Vite 8 + Tailwind 4 + marked/mermaid/highlight.js
- **IPC types**: ts-rs 10 + schemars; `bun run gen:types` → `src/lib/generated/`
- **Agent

…
## System Boundaries
| Boundary | Description | Direction |
|----------|-------------|-----------|
| Tauri IPC | Rust commands ↔ Svelte (`invoke` + streaming events); Rust types are the truth | internal |
| LLM Providers | OpenAI-compatible (per `OpenAiApiMode` routes `chat/completions` or `responses`) / Ollama local / Ollama Cloud (API key); Native used for lightweight phases and Litho fallback | outbound |
| 

…
## Code Map Index
| Concept | Location | Notes |
|---------|----------|-------|
| Asset generation pipeline | `crates/terrain-core/src/assets/mod.rs` | repomix/context/litho/sdd/ask/env aggregation |
| repomix pack | `crates/terrain-core/src/assets/repomix.rs` | `pack_agent_assets`, pack freshness |
| Context layering/generation | `crates/terrain-core/src/assets/context_layers.rs`, `agent_context.rs` | macro

…
## System
Boundaries


 —

 translate
 table
;
 update


 LLM Provider
 row
 
to

 include

 Ollama
 Cloud
 and
 per-provider
 profiles
.
 Keep

 trust
 
rules

 paragraph

 with

 thresholds
 verbatim-in
-translation
.
## Code
Map
 Index
 —


 translate
 table
;


 Litho
 row

 add
 `
litho
_native.rs`

 note
;
 new
 row


 `Model
 catalog
 &

 provider
 resolution

 |
 crates/terrain-agent
/src/model.rs

 |

 Ll
mProvider
,
 parse_provider
,
 list_provider_models
,
 ModelConfig

 resolution
`.


| 概念 | 位置 | 说明 |
|------|------|------|
| 知识工厂(核心) | `crates/terrain-core/src/assets/` | scan/pack/context/litho/incremental 状态机 |
| Repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | 生成 `agent/repomix.md`；`AGENT_PACK_STRATEGY` 声明行号可信 |
| Pack 读取/grep | `crates/terrain-core/src/assets/pack_read.rs`、`assets/query.rs` | 切片解析、`line_numbers_reliable`、命中回 `file_line` |
| 新鲜度评分/漂移 | `crates/terrain-core/src/freshness/` | scoring + drift_factors + codegraph + ledger |
| 仓库扫描器 | `crates/terrain-core/src/ingest/` | ProjectScanner / ScanReport / OpenApiImporter |
| Env 部署与状态 | `crates/terrain-core/src/integrations/` | plan/apply/env catalog/usage 探测 |
| IPC/Schema 类型 | `crates/terrain-core/src/schema/`、`ipc/` | ts-rs 导出源（`src/lib/generated/` 为产物） |
| 会话持久化 | `crates/terrain-core/src/sessions/` | Ask/SDD 会话存取 |
| Agent 运行时/引擎 | `crates/terrain-agent/src/runtime.rs`、`chat/` | ChatEngine、prompt、tracker、节流 |
| ACP 子进程 | `crates/terrain-agent/src/acp.rs`、`chat/acp.rs` | ACP 配置/spawn/纯 ACP 判定 |
| 工作流编排 | `crates/terrain-agent/src/workflows/` | ask / init / sdd / quick_refresh |
| 知识查询工具面 | `crates/terrain-agent/src/tools.rs` | read-context / read-doc / search_knowledge（结构化文档）/ grep-pack |
| 引用与源码定位 | `crates/terrain-core/src/source.rs`、`citations.rs` | 引文 path/行号 → 仓库内文件切片 |
| CLI 子命令 | `crates/terrain-cli/src/commands/` | assets/env/ask/sdd/tools/knowledge… |
| Tauri 命令层 | `src-tauri/src/commands/` | 暴露 core/agent 能力、payloads |
| 前端状态与 UI | `src/lib/stores/`、`src/lib/components/`、`markdownSanitize.ts`、`sourceRef.ts` | store 与面板；代码块外把 `path:line` 渲染为源码跳转按钮 |
