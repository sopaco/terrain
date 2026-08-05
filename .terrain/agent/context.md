---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手时代的**工程环境管理平台**：把 Git 仓库注册进来后，自动扫描代码、打包源码索引（repomix）、生成 C4 架构文档（Litho）、维护双轨知识资产（人类可读 `human/` + Agent 友好 `agent/`），并对外部 Coding Agent 提供 DeepWiki 知识问答与 SDD 标准开发工作流。知识存放于仓库内 `.terrain/`，随 Git 流转（"知识跟着代码走"）。消费者：桌面应用（Tauri + Svelte）、CLI（`terrain` 命令）、外部 Coding Agent（ACP 子进程 / `terrain tools`）。核心约束：Rust 为 IPC 唯一真源（ts-rs 生成 TS）、上下文硬上限 16 KiB、非确定性生成资产（Litho/context）禁自动合并、按包 grep 而非全量读码。

## 架构设计

```
┌────────────────────────────────────────────────────────────┐
│ 前端 Svelte 5 (src/)            Tauri IPC (src-tauri/src/) │
│  Ask/DeepWiki  SDD  Litho  环境  项目  用量  托盘/进度       │
└───────────────┬────────────────────────────────────────────┘
                │ commands / events (ts-rs 生成类型，Rust 真源)
┌───────────────▼────────────────────────────────────────────┐
│ terrain-core   领域核心（无执行）                              │
│  资产管线 assets/  检索 query/search  新鲜度 freshness/       │
│  摄取 ingest/     注册 registry      用量 usage   Schema      │
└───────────────┬────────────────────────────────────────────┘
┌───────────────▼────────────────────────────────────────────┐
│ terrain-agent 执行层                                        │
│  ChatEngine(Native LLM / ACP 双后端) 工具注册 上下文生成       │
│  Litho / SDD / Ask / Init / QuickRefresh 工作流               │
└───────────────┬────────────────────────────────────────────┘
                │ adk-model(openai/ollama)   agent-client-protocol(ACP 子进程)
┌───────────────▼───────────────┬───────────────┬────────────┐
│ repomix-core 源码打包           │ LLM Providers │ 外部 Agent │
│ codegraph(SQLite) / rtk        │ (OpenAI/Ollama)│ opencode   │
└───────────────────────────────┴───────────────┴────────────┘
```

分层：UI（Svelte）→ IPC 壳（Tauri commands）→ 核心（terrain-core，纯逻辑/资产，无 LLM 执行）→ 执行（terrain-agent，LLM/ACP/工具/工作流）。`terrain-cli` 是同一核心的终端入口；`terrain-ts-export` 是类型生成器（构建期）；`npm/` 发布各平台二进制的 JS shim；`env-catalog/` + `preset_skills/` 是给外部 Agent 的环境注入素材。关键依赖方向：terrain-agent 依赖 terrain-core；src-tauri 依赖两者。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| terrain-core | 领域核心：知识资产管线、三层检索、新鲜度、注册表、IPC 类型、Schema | `crates/terrain-core/src/` |
| ├ assets/ | repomix 打包、context/litho/sdd/ask 资产、增量刷新、上下文分层、环境集成 | `crates/terrain-core/src/assets/` |
| ├ freshness/ | git 比对 + codegraph drift 的保鲜评分 | `crates/terrain-core/src/freshness/` |
| ├ ingest/ | git 元数据 + OpenAPI 导入 + repomix 打包 | `crates/terrain-core/src/ingest/` |
| ├ ipc/ | Ask/Chat 会话与工作流事件的 IPC 类型 | `crates/terrain-core/src/ipc/` |
| └ schema/ | asset/doc/freshness/project/sdd/citation 结构体 | `crates/terrain-core/src/schema/` |
| terrain-agent | 执行层：ChatEngine（Native/ACP）、工具 schema/注册、上下文生成、Litho/SDD/Ask/Init 工作流 | `crates/terrain-agent/src/` |
| ├ chat/ | ChatEngine、native.rs/acp.rs 后端、prompt、tracker、types | `crates/terrain-agent/src/chat/` |
| └ workflows/ | ask / init / quick_refresh / sdd 编排 | `crates/terrain-agent/src/workflows/` |
| terrain-cli | CLI 入口：ask/env/init/knowledge/project/sdd/settings/source/tools/usage/assets | `crates/terrain-cli/src/commands/` |
| src-tauri | 桌面壳：Tauri commands、bundled tools、preset skills、托盘、构建 | `src-tauri/src/` |
| src/ (Svelte) | UI：Ask/DeepWiki、SDD、Litho、环境面板、用量、项目注册 | `src/lib/` |
| terrain-ts-export | 构建期 ts-rs 导出二进制（根类型 → `src/lib/generated/`） | `crates/terrain-ts-export/src/main.rs` |
| env-catalog | 环境集成模板：skills、agents-md 片段、工具 catalog | `env-catalog/` |
| preset_skills | 内置技能（ask/architecture/litho/sdd/agent-context） | `preset_skills/` |
| npm/ | 跨平台 CLI/RTK 二进制的 JS shim 与发布 | `npm/packages/` |
| agent-client-protocol-tokio-patched | ACP tokio 层的本地修补依赖（Cargo patch） | `crates/agent-client-protocol-tokio-patched/` |

## 核心流程

**1. 项目注册与扫描（init）**
1. `register_project` 将仓库登记到本地注册表 `~/.terrain/registry.json`（仅记录路径）。
2. `ProjectScanner` 采集 Git 元数据、按需导入 OpenAPI 规范。
3. repomix-core 将源码打包成 grep 友好的 `agent/repomix.md`（`pack_agent_assets`）。
4. 初始化 `.terrain/` 知识结构（agent/、knowledge/、human/），必要时生成 agent/context.md。

**2. DeepWiki 知识问答（Ask，三层检索）**
1. Macro：预载 `agent/context.md`（含项目概览/架构/模块地图）。
2. Meso：按需 `read_agent_context(section=…)` 或搜索 `human/`、`knowledge/` 文档（`extract_context_section`）。
3. Micro：`grep-pack` / `read-pack-file` 查源码；结果带 `SourceCitation` 与工具调用 trace。
4. 由 ChatEngine 执行：`AcpNative`（内置 LLM）或 ACP 子进程（opencode，走 agent-client-protocol）；LLM 不可用时降级到纯检索 `fallback_search_reply`。

**3. Litho C4 文档生成**
1. 预处理（phase1）：收集源码/元数据（`prepare_litho_generation`）。
2. C4 研究（phase2）：产出研究产物，持久化到 `.terrain/.litho-agent/` 支持中断恢复。
3. 编排/组合（phase3）：撰写六份标准人类文档（含 Mermaid 图表）。
4. 输出（phase4）：写回 `human/`，更新 meta/索引。`LithoRunMode::{Auto, FullRebuild}` 决定增量或全量。

**4. SDD 标准开发工作流**
1. 需求（Requirements）→ `1.requirements.md`（Native LLM）。
2. 技术设计（Design）→ `2.tech-design.md`（Native LLM）。
3. 代码生成（Codegen）→ `3.implementation.md` + 仓库改动（委托 ACP Agent）。
4. 代码审查（Review）→ 审查产物。每阶段产出可审查 Markdown（`run_sdd_phase` 按阶段分发到 LLM/ACP）。

**5. 环境集成（Env）**
1. 探测（probe）当前 Skills / CLI 工具 / AGENTS.md 状态（`EnvStatus`）。
2. 规划（plan）差异 → `EnvPlan`/`EnvPlanStep`。
3. 应用（apply）安装 terrain-knowledge/repomix/codegraph/rtk 技能、CLI 工具与 `AGENTS.md` 片段（`agent_tools_deploy`、`bundled_tools`）。

**6. 知识保鲜（Freshness）**
1. 基于 git 比对 + codegraph drift 交叉验证计算 `freshness_score`。
2. 分数低于阈值触发知识资产刷新（`compute`/`scoring`/`drift_factors`/`ledger`）。

## 技术选型

- **语言**：Rust（edition 2024，workspace 多 crate）+ TypeScript/Svelte 5。
- **桌面壳**：Tauri 2（`src-tauri/`，capabilities 权限声明，托盘 + 多窗口 Usage）。
- **前端**：Svelte 5（runes）+ Vite 8 + Tailwind CSS 4 + marked/mermaid/highlight.js。
- **IPC 类型**：ts-rs（`terrain-ts-export` 二进制）生成 `bindings/*.ts` + `src/lib/generated/`；`bun run gen:types`。
- **LLM 执行**：adk-* 家族（adk-model 支持 openai/ollama 的 Native 后端）+ agent-client-protocol（ACP 子进程），opencode feature 门控。
- **源码索引**：repomix-core（Rust 锈化实现）打包 `agent/repomix.md`；CodeGraph（SQLite 符号图）、RTK（shell 输出压缩）。
- **发布**：npm 包（`npm/packages/cli|rtk` + 平台二进制 shims）、`scripts/cross-windows-terrain.sh` 交叉编译。
- **基础库**：tokio、serde/schemars、anyhow/thiserror、tracing、chrono、walkdir/ignore、futures、slug。

## 系统边界

| 边界 | 说明 | 方向 |
|------|------|------|
| Tauri IPC | Rust commands ↔ Svelte（`invoke` + 流式事件），Rust 类型为唯一真源 | 内 |
| ACP 子进程 | 外部 Coding Agent（opencode）经 agent-client-protocol 调用，执行 SDD 代码生成 / Litho / Ask；本地 patch 了 tokio 层 | 出 |
| LLM Providers | OpenAI 兼容 / Ollama（adk-model），Native 轻量阶段（Ask 摘要、SDD 文档）使用 | 出 |
| 本地注册表 | `~/.terrain/registry.json`（仅路径，无正文）；知识正文只在仓库 `.terrain/` | 本地 |
| 知识文件系统 | `.terrain/agent/`（生成、禁合并）、`human/`（生成）、`knowledge/`（人工）、`.litho-agent/`（研究中产物） | 本地 |
| repomix-core | 生成源码索引包（370 文件/318k tokens 级） | 出 |
| CodeGraph / RTK | 外部索引与工具二进制，随项目分发（`packages/`、`~/.terrain/bin/`） | 出 |
| Git | 摄取元数据、保鲜比对（baseline HEAD）、git 策略（.gitattributes 声明 -merge） | 出 |
| 外部命令 | `terrain tools`（ACP 模式 JSON stdout：pack-meta/grep-pack/read-pack-file/read-context） | 出 |

信任边界：前端不信任 Rust 返回值（Rust 校验）；ACP 子进程视为外部实体（可 spawn 任意命令，故经 ACP 授权配置门控）；生成资产（context/litho/human）非确定性 → 冲突时禁手工合并，保留任一版本后重跑 scan。IPC `Option<T>` → `T | null`，前端判空按此契约。

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 资产管线总入口 | `crates/terrain-core/src/assets/mod.rs` | repomix/context/litho/sdd/ask/env 聚合 |
| repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | `pack_agent_assets`、新鲜度判定 |
| 上下文分层 | `crates/terrain-core/src/assets/context_layers.rs` | macro/meso/on-demand 切片 |
| Ask 检索 | `crates/terrain-core/src/assets/ask.rs` + `crates/terrain-agent/src/workflows/ask.rs` | 三层检索 + fallback |
| 增量刷新 | `crates/terrain-core/src/assets/incremental.rs`、`crates/terrain-agent/src/workflows/quick_refresh.rs` | |
| Litho 生成 | `crates/terrain-core/src/assets/litho.rs` + `crates/terrain-agent/src/litho.rs` | 四阶段、`.litho-agent/` 恢复 |
| SDD 工作流 | `crates/terrain-agent/src/workflows/sdd.rs` + `src/lib/components/SddWorkflowPanel.svelte` | 阶段分发 LLM/ACP |
| ChatEngine | `crates/terrain-agent/src/chat/mod.rs` | Native/ACP 后端选择 |
| 工具 schema/注册 | `crates/terrain-agent/src/tool_schema.rs`、`tools.rs`、`compat_tool.rs` | AgentToolPaths |
| 新鲜度 | `crates/terrain-core/src/freshness/`（compute/scoring/drift/codegraph/git） | |
| 注册表/项目 | `crates/terrain-core/src/registry.rs`、`project.rs` | |
| 环境集成 | `crates/terrain-core/src/assets/env/`、`agent_tools_deploy.rs`、`bundled_tools.rs` | EnvPlan/status |
| Tauri commands | `src-tauri/src/commands/` | project/sessions/workflows/knowledge/env/usage/assets |
| 前端 IPC 封装 | `src/lib/api.ts`、`types.ts` | invoke 封装 + 类型入口 |
| 类型导出 | `crates/terrain-ts-export/src/main.rs` | ts-rs 根类型汇总 |
| ACP 协议 patch | `crates/agent-client-protocol-tokio-patched/src/acp_agent.rs` | Cargo patch |
| 内置技能/环境模板 | `preset_skills/`、`env-catalog/` | 注入外部 Agent |