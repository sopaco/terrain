---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是一款 **AI 友好的工程环境管理器**（Rust workspace 版本 0.9.8），形态为 **Tauri 2 桌面应用 + `terrain` CLI**。核心主张：从 Git 仓库生成并持续保鲜三类知识资产，让 Coding Agent 开箱可用——

- `human/`：面向人的叙述性架构文档（Litho 生成）
- `.terrain/agent/context.md`：面向 Agent 的结构化架构上下文（≤14 KiB）
- `.terrain/agent/repomix.md`：源码索引包，按需 grep/读取

消费者：桌面前端（阅读器 / DeepWiki 问答）、外部编码 Agent（ACP 子进程 + `terrain tools` JSON 契约）、CLI/CI。关键约束：**凡可离线完成的能力绝不依赖 LLM**；生成式任务永远"可复核、可降级、失败不覆盖好资产"；资产随仓库入库、生成物禁用自动合并。

## 架构设计

分层单向依赖：`terrain-core ← terrain-agent ← {terrain-cli, src-tauri}`；前端只经 IPC 与 Rust 通信，类型由 ts-rs 从 Rust 导出。

| 容器 | 职责 | 关键依赖 |
|------|------|----------|
| `terrain-core` | 领域内核：扫描、repomix 打包、检索、新鲜度、环境集成、会话、Schema。**零 LLM** | repomix-core、git、walkdir、sled 类本地索引 |
| `terrain-agent` | 智能编排层：ChatEngine、ACP 客户端、工具注册、工作流 | `adk-agent/core/tool/model/runner/session` 2.2、`agent-client-protocol` 1.3 |
| `terrain-cli` | `clap` 子命令：scan/init/refresh/ask/sdd/tools/assets/env/usage | core + agent |
| `src-tauri` | 桌面壳：窗口、托盘、Tauri commands（IPC 边界）、`AppState{runtime: Runtime}` | tauri 2.11、plugin-dialog/shell |
| `src/`（Svelte） | 七大面板 UI，Svelte 5 runes + Vite 8 + Tailwind 4 | marked、mermaid、highlight.js、html2canvas |
| `terrain-ts-export` | ts-rs 导出二进制，产出 `src/lib/generated/`（**禁止手改**） | ts-rs 10 |
| `env-catalog/` + `npm/` | Agent 环境资产（AGENTS.md 片段、Skills、工具模板）与 CLI/工具二进制分发 | npm workspace、平台包 |

**两条执行路径**：`terrain-core` 的纯确定性流水线（scan → pack → search → freshness → env）与 `terrain-agent` 的生成流水线（Litho 文档、Agent 上下文、DeepWiki 问答、SDD）。后者按 `AgentExecution` 选择**原生 LLM**、**纯 ACP**（外部 agent 必在 PATH）或**混合 `AcpNative`**（原生 LLM 兜底，ACP 可选）。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| 扫描与打包 | Git 仓库走查、`index.md`/`modules/`/`interfaces/` 生成、repomix 打包 | `crates/terrain-core/src/ingest/`、`src/assets/repomix.rs`、`repo_walk.rs` |
| 知识资产编排 | Litho 计划/进度、增量 diff 更新、meta 收集、上下文层切分 | `crates/terrain-core/src/assets/`（`litho.rs`、`incremental.rs`、`project_meta.rs`、`context_layers.rs`） |
| 新鲜度 | Git 漂移、CodeGraph 漂移检测、评分与阈值、账本 | `crates/terrain-core/src/freshness/` |
| 检索 | 全文检索、pack grep/read、知识文档读取、引用解析 | `src/search.rs`、`src/assets/query.rs`、`src/assets/pack_read.rs`、`src/citations.rs` |
| 环境集成 | Skills 部署到项目内 `.agents/skills/` 与 `.claude/skills/`，工具与 env catalog 部署到 `~/.terrain/`；AGENTS.md 打补丁、env 探测 | `src/assets/env/`、`src/integrations/`、`src/agent_tools_deploy.rs`、`env-catalog/` |
| Agent 运行时 | ChatEngine（native/ACP）、Runtime 缓存、工具注册、schema | `crates/terrain-agent/src/{runtime,acp,chat,tools,tool_schema}.rs` |
| 工作流 | init、ask、sdd、quick_refresh、Litho 上下文生成 | `crates/terrain-agent/src/workflows/`、`agent_context.rs`、`context_generator.rs` |
| CLI | 子命令入口、JSON 输出（ACP 契约）、util | `crates/terrain-cli/src/`（`cli.rs`、`commands/`） |
| 桌面壳 | Tauri commands、托盘、bundled tools、preset skills 注入 | `src-tauri/src/`（`commands/`、`tray.rs`、`bundled_tools.rs`） |
| 前端 UI | 概览/知识/DeepWiki/环境/SDD/用量/设置面板、状态 stores、i18n | `src/App.svelte`、`src/lib/components/`、`src/lib/stores/`、`src/lib/i18n/` |
| 类型桥 | IPC 类型真源与导出 | `src/schema/`、`src/ipc/`、`src/ts_ipc.rs`、`crates/terrain-ts-export/` |
| 分发 | npm 包装、平台二进制、版本同步 | `npm/packages/`、`npm/scripts/`、`packages/` |

## 核心流程

**1. 项目初始化（`initialize_project_cmd` / `terrain init`）**
1. 校验仓库路径并写入 `~/.terrain/registry.json`，解析 `KnowledgePaths`（仓库内 `.terrain/`）。
2. `ProjectScanner` 走查仓库 → 生成 `index.md`、`modules/`、`interfaces/` 等结构文档。
3. 若 human 文档不完整 → 按 Litho 计划分批生成（研究 → 逐模块 → 汇总），带进度事件。
4. repomix 打包 → `agent/repomix.md` + `agent/meta.json`（含 baseline git HEAD）。
5. 生成 `agent/context.md`：先采集 `terrain-meta.json` 输入（`agent/meta-inputs.md`），再按宏/中/微三层要求生成并**截断至 16 KiB 上限**。
6. 任一步失败只写 note，不覆盖已存在的有效资产。

**2. DeepWiki 问答（三层检索）**
1. 取 `context.md` 的**宏层**（概览 + 章节索引，≤4,500 字符）预载入 prompt；分数低于 50 则**不预载**。
2. 模型按需调用 `read_agent_context(section=…)` 取**中观层**（单次 ≤3,500 字符）。
3. 定位实现时调用 `grep_agent_pack` → `read_agent_pack_file`（**微观层**，默认 150 行/次），带 tool-session 缓存与重复调用短路。
4. 输出经引用抽取/合并，前端渲染带源码切片的回答；LLM 不可用时降级为纯检索答复。

**3. 新鲜度计算与信任降级**
1. 以资产记录的 baseline HEAD 为基准，统计提交漂移（2 分/提交，上限 40）与改动文件比例（上限 30）。
2. 叠加同步时长（1 分/天，上限 5）与脏工作区（5 分）；**不可测漂移按饱和惩罚**计入（fail-closed）。
3. `context.md` 为派生资产，分数 ×0.9 折扣且不高于其源码包分数。
4. 阈值：≥80 新鲜（绿）、≥70 可验证（黄，须交叉核对 repomix）、<50 禁止预载宏观上下文；结果回写 `.terrain/.meta/freshness.json` 账本。

**4. ACP 模式上下文生成**
1. Terrain 注入 `TERRAIN_AGENT_ARCH_SKILL`、`TERRAIN_ASK_SKILL` 与项目 slug/env，拉起外部 agent 子进程。
2. 外部 agent 全程只用 `terrain tools` 子命令（`pack-meta`、`grep-pack`、`read-pack-file`）**读取打包副本**，不直接读活仓库。
3. Terrain 把回复原文**整体**持久化为 `context.md`——只回摘要会覆盖好文档，因此契约要求"回复即文件"。

## 技术选型

- **语言/构建**：Rust edition 2024，MSRV 1.94，Cargo workspace（5 members）；release `lto=thin`、`strip`、`codegen-units=1`
- **Agent 框架**：`adk-agent/adk-core/adk-tool/adk-model(openai+ollama)/adk-runner/adk-session` 2.2
- **协议**：`agent-client-protocol` 1.3（ACP 子进程客户端）、JSON stdout 工具契约
- **源码索引**：`repomix-core` 2.0（repomix-rs）
- **类型桥**：`ts-rs` 10（`serde-compat`，camelCase 重命名），`bun run gen:types`
- **桌面**：Tauri 2.11（dialog/shell 插件）、系统托盘、sidecar 资源打包
- **前端**：Svelte 5.56（runes）、Vite 8、TypeScript 5.9、Tailwind 4、`marked`/`mermaid`/`highlight.js`/`html2canvas`
- **CLI/工具链**：`clap`（多子命令 + `ValueEnum`）、`tokio`、`serde`/`serde_json`/`serde_yaml`/`schemars`、`tracing`、`walkdir`/`ignore`
- **分发**：npm 平台包（`cli-darwin-arm64`、`cli-win32-x64`、`rtk-*`），版本同步与 shim 生成脚本；i18n 一致性由 `scripts/check-i18n-parity.ts` 守护

## 系统边界

| 边界 | 内容 | 说明 |
|------|------|------|
| 仓库内资产 | `{repo}/.terrain/{agent,human,knowledge}/`、`.terrain/.meta/` | 入库；生成物 `-merge` 禁自动合并；`repomix*`、`meta.json` 等本机衍生物不入库 |
| 用户主目录 | `~/.terrain/registry.json`（仅项目指针）、`~/.terrain/bin/`（rtk/codegraph/terrain）、env catalog | 由环境集成部署，不含知识正文；Skills 在项目内 `.agents/skills/` 与 `.claude/skills/` |
| 外部进程 | ACP agent（默认 `opencode`，可用 `TERRAIN_ACP_*` 覆盖）、`git`、`codegraph`、`rtk`、`bun`/`npx` | 必须经 `terrain_core::process` 创建，保证 Windows 无控制台闪窗 |
| LLM 提供方 | OpenAI 兼容端点、Ollama（经 adk-model） | 配置存本地 model settings；可用 `llm_status` 探测 |
| 第三方服务 | GitHub（仓库/Release 分发）、npm registry、DeepWiki 上游（Litho 谱系） | 无运行时强依赖，离线可用 |
| 信任边界 | 外部 Agent 只通过 `terrain tools` JSON 读资产，不直接写；`knowledge/` 视为人为维护；前端只信 Rust 生成的 IPC 类型 | 路径经 `validate_repo` 校验，拒绝仓库外写入 |

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 领域内核入口与公开 API | `crates/terrain-core/src/lib.rs` | 全部模块 re-export，先读此文件 |
| 知识路径解析 | `crates/terrain-core/src/paths.rs` | `KnowledgePaths`：`{repo}/.terrain/` |
| 项目注册表 | `crates/terrain-core/src/registry.rs` | `~/.terrain/registry.json` |
| 仓库扫描 | `crates/terrain-core/src/ingest/mod.rs`、`ingest/git.rs` | `ProjectScanner::scan_repo` |
| repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | feature `repomix` |
| Agent 上下文生成/校验 | `crates/terrain-core/src/assets/agent_context.rs` | baseline HEAD 比对 |
| 上下文分层与限额 | `crates/terrain-core/src/assets/context_layers.rs` | 16 KiB / 4,500 / 3,500 |
| 增量更新规划 | `crates/terrain-core/src/assets/incremental.rs` | git diff 驱动 |
| 开发者 meta | `crates/terrain-core/src/assets/project_meta.rs` | `terrain-meta.json` |
| 新鲜度评分 | `crates/terrain-core/src/freshness/{scoring,compute,codegraph,drift_factors}.rs` | 阈值 80/70/50 |
| 环境集成 | `crates/terrain-core/src/assets/env/`、`src/integrations/mod.rs` | plan → apply → status |
| Agent 运行时与 ACP | `crates/terrain-agent/src/{runtime,acp,chat/acp.rs,chat/native.rs}.rs` | `ChatEngine` 缓存策略 |
| Agent 工具集 | `crates/terrain-agent/src/tools.rs` | `list_projects`、`read_agent_context`、`grep_agent_pack` |
| 工作流编排 | `crates/terrain-agent/src/workflows/{init,ask,sdd,quick_refresh}.rs` | SDD 三阶段 Requirements→TechDesign→CodeGen |
| CLI 子命令 | `crates/terrain-cli/src/cli.rs`、`commands/` | `terrain tools` 为 ACP 契约 |
| Tauri IPC | `src-tauri/src/commands/`、`src-tauri/src/lib.rs` | `generate_handler!` 注册全部命令 |
| 前端状态与 IPC 封装 | `src/lib/{agentExecution,appBootstrap,askSession}.ts`、`src/lib/stores/` | runes 化 store |
| 生成类型（勿手改） | `src/lib/generated/`、`crates/terrain-ts-export/src/main.rs` | `bun run gen:types` |