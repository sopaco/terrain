---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是一个**面向编码 Agent 的工程环境平台**（Sopaco 开源；repomix-rs 驱动源码打包）。标语：*"Terrain 铺好地面，让 Agent 不必猜测该站在哪里。"* 指向一个 Git 仓库后，它会扫描代码、打包源码（repomix）、生成 Agent context、C4 架构文档（Litho）、双轨知识（`human/` + `agent/`）、知识保鲜追踪，并向外部 Coding Agent 暴露 Ask 问答与四阶段 SDD 工作流。知识保存在仓库内 `.terrain/` 目录并随 Git 分支流转。消费方：Tauri 桌面应用（Svelte UI）、CLI（`terrain` / `terrain tools`）、以及通过 ACP 子进程接入的外部 Agent。约束：Rust 是 IPC 唯一真源（ts-rs → TypeScript）；`context.md` 硬上限 16 KiB；生成资产非确定性（重新生成而非手工合并）；Agent 查询 repomix pack 而非实时文件系统。

## 架构设计

```
┌───────────────────────────────────────────────────────────────┐
│ Svelte 5 前端 (src/)            Tauri 2 壳 (src-tauri/)       │
│  Ask/DeepWiki · SDD · Litho · Env · Projects · Usage · Tray   │
└───────────────┬───────────────────────────────────────────────┘
                │ invoke + 流式事件 (ts-rs; Rust 是 IPC 真源)
┌───────────────▼───────────────────────────────────────────────┐
│ terrain-core — 领域核心（不执行 LLM）                           │
│  assets/ · search/query · freshness · ingest · registry       │
│  sessions · ipc+schema 类型 · env 集成                         │
└───────────────┬───────────────────────────────────────────────┘
┌───────────────▼───────────────────────────────────────────────┐
│ terrain-agent — 执行层                                        │
│  ChatEngine（Native ADK / ACP）· tools · context 生成          │
│  workflows: Ask · Init · SDD · QuickRefresh                   │
└───────────────┬───────────────────────────────────────────────┘
                │ adk-model (OpenAI/Ollama) · agent-client-protocol
┌───────────────▼───────────────┬───────────────┬───────────────┐
│ repomix-core · CodeGraph      │ LLM 提供方     │ ACP Agent     │
│ RTK · bundled CLI shims       │ OpenAI/Ollama │ (opencode)    │
└───────────────────────────────┴───────────────┴───────────────┘
```

| 层 | 职责 | 关键路径 |
|-------|------|-----------|
| UI | 面板、stores、i18n（en/zh-CN） | `src/`、`src/lib/api.ts` |
| IPC 壳 | Tauri 命令、tray、捆绑工具 | `src-tauri/src/commands/` |
| 领域核心 | 资产生成、三层检索、保鲜、ingest | `crates/terrain-core/` |
| 执行 | ChatEngine、workflows、Litho/SDD 驱动、工具注册表 | `crates/terrain-agent/` |
| 入口 | 桌面应用、`terrain-cli`、npm shims（`cli`/`rtk`） | 共享 core + agent |

- **依赖方向**：terrain-agent → terrain-core；src-tauri 与 terrain-cli → 两者；ACP 运行时直接依赖 `agent-client-protocol`（adk-acp），无本地 patch crate。
- **类型流**：ts-rs（`ts-export` feature）→ `terrain-ts-export` → `src/lib/generated/`（`bun run gen:types`）。
- **设计原则**：把知识逻辑（core）与 LLM/ACP 执行（agent）分离。

## 模块地图

| 模块 | 职责 | 主要路径 |
|--------|----------------|---------------|
| terrain-core | 领域核心：资产生成、三层检索、保鲜、ingest、registry、IPC 类型 | `crates/terrain-core/src/` |
| assets/ | repomix pack、agent context、Litho/SDD/Ask 资产、增量刷新、env 集成 | `crates/terrain-core/src/assets/` |
| freshness | Git + CodeGraph 漂移评分、基线台账、fail-closed 漂移判定（基准不可达/缺失 → stale） | `crates/terrain-core/src/freshness/` |
| ingest | 项目扫描、Git 元数据、OpenAPI 导入 | `crates/terrain-core/src/ingest/` |
| terrain-agent | ChatEngine、workflows、Litho/SDD 驱动、ACP/native 后端、工具注册表 | `crates/terrain-agent/src/` |
| chat/ | 双后端：Native ADK Runner + ACP 子进程 | `crates/terrain-agent/src/chat/` |
| workflows/ | Ask、Init、SDD、QuickRefresh 编排 | `crates/terrain-agent/src/workflows/` |
| terrain-cli | 无头入口：scan、init、ask、tools、env、usage | `crates/terrain-cli/src/commands/` |
| src-tauri | 桌面壳：IPC 命令、tray、preset skills、env catalog | `src-tauri/src/` |
| 前端 | Svelte 5 UI、IPC 封装、stores、i18n | `src/lib/` |
| preset_skills | 捆绑的 Agent skills（Litho、SDD、Ask、arch、context） | `preset_skills/` |
| env-catalog | Agent 工具链目录、AGENTS.md 片段、skill 模板 | `env-catalog/` |

## 核心流程

**1. 项目登记 → 知识资产生成**
1. `initialize_project` 将仓库登记到 `~/.terrain/registry.json`（仅路径指针）。
2. `ProjectScanner` 采集 Git 元数据；可选 OpenAPI 导入（`scan_project`）。
3. repomix-core 打包源码 → `.terrain/agent/repomix.md`（`pack_agent_assets`）。
4. LLM 生成 `agent/context.md`；可选 Litho 四阶段运行产出 `human/` C4 文档（产物保留在 `.litho-agent/` 便于续传）。
5. 写入保鲜基线台账；后续用 git/codegraph 交叉校验漂移。增量 context 刷新依赖 `agent_context_recorded_baseline_head`（`context-meta.json`）——仅 repack 不意味着 `context.md` 已同步。漂移度量 **fail-closed**：记录在案的基准提交不可达（`baseline_unreachable`，如 rebase/squash/amend/force-push/浅克隆）或仓库内就绪资产未记录基准（`baseline_missing`）时按 stale 计分，不静默判定为"零漂移"。三层资产明细分别计分，`overall_score` 取三者最小值；`drift_factors` 给出可解释的扣分原因。

**2. Ask 知识问答（DeepWiki，三层检索 + 双后端）**
1. Macro：预载 `agent/context.md` 概览/架构/模块地图。
2. Meso：按需 `read_agent_context(section=…)` 或检索 `human/`、`knowledge/` 文档。
3. Micro：`grep_agent_pack` → `read_agent_pack_file` 获取源码切片。
4. `ChatEngine` 执行：Native（ADK Runner；按 `OpenAiApiMode` 走 OpenAI `chat/completions` 或 `responses`，或 Ollama）或 ACP 子进程（opencode）；由 `AcpSettings` 门控；LLM 不可用时回退 `fallback_search_reply`。
5. 流式推送 thinking/tool_calls/phase/usage 事件（`AskStreamEvent`）+ 来源引用；可选会话持久化。

**3. SDD 四阶段开发**
1. Requirements → `1.requirements.md`。
2. TechDesign → `2.tech-design.md`。
3. Codegen → `3.implementation.md` + 仓库改动（委派给 ACP Agent）。
4. CodeReview。文档阶段用 Native LLM；代码阶段用 ACP（`run_sdd_phase` 按阶段派发）。

**4. 环境集成（Env）**
1. 探测 Skills / CLI 工具 / AGENTS.md 状态（`EnvStatus`）。
2. 规划差异 → `EnvPlan` / `EnvPlanStep`。
3. 应用：部署 terrain-knowledge/repomix/codegraph/rtk skills、捆绑工具、`AGENTS.md` 片段。

## 技术选型

- **Rust**：workspace（terrain-core、terrain-agent、terrain-cli、terrain-ts-export、src-tauri），edition 2024，rust-version 1.94
- **桌面壳**：Tauri 2（capabilities ACL、plugin-dialog/shell、tray + Usage 窗口）
- **前端**：Svelte 5（runes）+ Vite 8 + Tailwind 4 + marked/mermaid/highlight.js
- **IPC 类型**：ts-rs 10 + schemars；`bun run gen:types` → `src/lib/generated/`
- **Agent 运行时**：ADK Rust 2.2（adk-core/agent/runner/session/tool/model + adk-acp）+ agent-client-protocol 1.3（ACP），无本地 patch crate；`OpenAiApiMode` 路由 chat 与 responses API
- **源码索引**：repomix-core 2.0（repomix-rs）→ `agent/repomix.md`；CodeGraph（SQLite 符号图）；RTK 压缩 shell 输出
- **存储**：`.terrain/`（版本化知识）、`~/.terrain/registry.json`（项目指针）、`.codegraph/`（本地索引）
- **分发**：npm 包（`cli`/`rtk` + darwin-arm64/win32-x64 shims）、Tauri bundle；release profile `lto=thin`、`strip=true`
- **基础库**：tokio、serde/serde_json、anyhow/thiserror、tracing、chrono、walkdir/ignore、futures

## 系统边界

| 边界 | 描述 | 方向 |
|----------|-------------|-----------|
| Tauri IPC | Rust 命令 ↔ Svelte（`invoke` + 流式事件）；Rust 类型为真源 | 内部 |
| LLM 提供方 | OpenAI 兼容（按 `OpenAiApiMode` 走 `chat/completions` 或 `responses`）/ Ollama；Native 用于轻量阶段 | 出 |
| ACP 子进程 | 经 agent-client-protocol 接入外部 Coding Agent（opencode）；`acp_config_json` 注入配置；可拉起任意命令 → 信任边界，由 `AcpSettings` 门控 | 出 |
| 本地 registry | `~/.terrain/registry.json` 仅存项目路径，不存知识正文 | 本地 |
| 知识文件系统 | `.terrain/agent/`（生成）、`human/`（生成）、`knowledge/`（手工）、`.litho-agent/`（研究）、`repomix.md`（本地索引） | 本地 |
| 外部代码 | 只读扫描/打包（git、OpenAPI、repomix）；不写目标仓库（SDD Codegen 除外） | 出 |
| 工具二进制 | CodeGraph / RTK / terrain CLI（`packages/`、`~/.terrain/bin/`、npm shims） | 出 |
| Git | ingest、保鲜基线、`.gitattributes` 把生成资产标记为 `-merge` | 出 |

信任规则：前端对 IPC `Option<T>` → `T | null` 判空；ACP 子进程为外部方；生成资产非确定性——重新生成而非手工合并；`grep-pack`/`read-pack-file` 是 Agent 获取源码的唯一入口（非实时文件系统）。保鲜阈值（`freshness/mod.rs`）：`FRESH_THRESHOLD = 80`（UI 绿色新鲜态）、`VERIFY_THRESHOLD = 70`（在此区间需用 repomix 交叉验证架构论断）、`MACRO_PRELOAD_THRESHOLD = 50`（低于此不预载 macro 架构上下文）；基准不可达/缺失一律按 stale 处理。

## 代码映射索引

| 概念 | 位置 | 备注 |
|---------|----------|-------|
| 资产生成流水线 | `crates/terrain-core/src/assets/mod.rs` | repomix/context/litho/sdd/ask/env 聚合 |
| repomix pack | `crates/terrain-core/src/assets/repomix.rs` | `pack_agent_assets`、pack 保鲜 |
| Context 分层/生成 | `crates/terrain-core/src/assets/context_layers.rs`、`agent_context.rs` | macro/meso 切片；baseline-head 刷新 |
| Context 生成器（agent） | `crates/terrain-agent/src/context_generator.rs`、`agent_context.rs` | LLM 驱动的 context 合成 |
| 增量刷新 | `crates/terrain-core/src/assets/incremental.rs`、`crates/terrain-agent/src/workflows/quick_refresh.rs` | 增量更新 |
| Litho 生成 | `crates/terrain-core/src/assets/litho.rs`、`crates/terrain-agent/src/litho.rs` | 四阶段、`.litho-agent/` 续传 |
| SDD 工作流 | `crates/terrain-agent/src/workflows/sdd.rs`、`crates/terrain-agent/src/sdd.rs` | 按阶段派发 LLM/ACP |
| Ask 检索 | `crates/terrain-core/src/assets/ask.rs`、`crates/terrain-agent/src/workflows/ask.rs` | 三层检索 + fallback |
| ChatEngine 双后端 | `crates/terrain-agent/src/chat/mod.rs`、`native.rs`、`acp.rs` | ADK Runner / ACP |
| 知识检索与文档读取 | `crates/terrain-core/src/search.rs` | `KnowledgeSearch`；`read_doc_at` |
| 保鲜 | `crates/terrain-core/src/freshness/` | compute/scoring/git/codegraph/ledger/drift_factors；fail-closed 漂移基准（`baseline_unreachable`/`baseline_missing`） |
| Env 集成 | `crates/terrain-core/src/assets/env/`、`agent_tools_deploy.rs` | EnvPlan/Status、工具链部署 |
| IPC 类型 | `crates/terrain-core/src/schema/`、`ipc/` | ts-export 注解 |
| Tauri 命令层 | `src-tauri/src/commands/` | project/sessions/workflows/knowledge/env/usage |
| 前端 IPC 封装 | `src/lib/api.ts`、`types.ts`、`appBootstrap.ts` | invoke + bootstrap 单例去重 |
| CLI + Ask 工具 | `crates/terrain-cli/src/commands/tools.rs` | `terrain tools` 知识层 |
```