---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 把本地 Git 仓库变成一张**人类与 Agent 共享的地图**：在仓库内生成 `.terrain/` 知识层，包含 C4 人类文档、Agent 宏观上下文 `agent/context.md`、可按需 grep 的源码包 `agent/repomix.md`，以及记录"知识落后代码多少"的新鲜度账本。

- **消费方**：Coding Agent（通过 `terrain tools` CLI 或 MCP）、桌面阅读器用户、CI 流水线。
- **关键约束**：凡是能离线完成的绝不调用 LLM——扫描、打包、检索、评分、环境探测均为确定性文件与 Git 操作（`terrain-core` 无 HTTP 客户端依赖）；只有写文档、写上下文、答问题才走 LLM 或外部 ACP Agent。
- 知识资产以 Git 提交为基线随代码演进，仓库内 `.terrain/` 可入库、可 diff。

## 架构设计

三层容器 + 一条类型单向流：

```
Svelte 前端 (src/)  ──invoke──▶  src-tauri (commands)  ──▶  terrain-agent  ──▶  terrain-core
        ▲                                                      │                    │
        └──────── terrain-cli (crates/terrain-cli) ────────────┘                    ▼
                                                                           .terrain/ 知识资产
```

| 层 | 职责 | 主要依赖 |
|---|---|---|
| `terrain-core` | 纯确定性内核：路径约定、扫描索引、repomix 打包、pack 读取/grep、freshness、schema、registry、进程与平台抽象 | serde、walkdir、git 子进程、`process` 封装 |
| `terrain-agent` | 非确定性层：ChatEngine（native LLM / ACP 双通道）、工具 schema、Litho 文档流水线、SDD、模型设置 | `adk-*` 2.2、`agent-client-protocol` 1.3、`terrain-core` |
| `terrain-cli` | 无头入口，子命令 `ask / init / knowledge / tools / env / sdd / settings / source / assets / project / usage` | `terrain-agent` + `terrain-core` |
| `src-tauri` | 桌面壳：Tauri commands 聚合、托盘、bundled tools 部署、env catalog | `terrain-*` crates |
| `src/` (Svelte) | 阅读器与交互层：Overview / Ask / SDD / Env / Usage 面板、Markdown 与 Mermaid 渲染 | Tauri `invoke`、Svelte 5 runes |

**类型契约**：Rust 是 IPC 唯一真源 → `crates/terrain-ts-export` 导出 → `src/lib/generated/`（禁止手改）。`Option<T>` 生成 `T | null`；UI 专用扩展放 `src/lib/types.client.ts`，经 `src/lib/types.ts` 统一 re-export。

**分层读取模型**（Ask 消费方式）：Macro = `context.md` 概览预载；Meso = `terrain tools read-context --section`；Micro = `grep-pack` → `read-pack-file`。Agent 永不读活仓库文件系统，repomix 包是唯一代码真源。

## 模块地图

| 模块 | 职责 | 主要路径 |
|---|---|---|
| 路径与项目注册 | `.terrain/` 目录布局、仓库路径解析、全局 registry | `crates/terrain-core/src/paths.rs`、`project.rs`、`registry.rs` |
| 扫描与索引 | 走查仓库、识别技术栈、生成 `index.md` 与 `interfaces/` | `crates/terrain-core/src/ingest/`、`repo_walk.rs` |
| Repomix 打包 | 生成压缩源码包与 `pack-meta`，增量刷新 | `crates/terrain-core/src/assets/repomix.rs`、`incremental.rs` |
| 包读取与检索 | `grep-pack` / `read-pack-file`、知识全文检索 | `crates/terrain-core/src/assets/pack_read.rs`、`search.rs`、`query.rs` |
| Agent 上下文资产 | `agent/context.md` 生成与就绪判定、macro/meso/micro 分层 | `crates/terrain-core/src/assets/agent_context.rs`、`context_layers.rs` |
| Freshness | Git 漂移测量、因子归因、评分、ledger 回写 | `crates/terrain-core/src/freshness/` |
| 人类文档 (Litho) | 四阶段 C4 流水线与运行态 | `crates/terrain-agent/src/litho.rs`、`litho_native.rs`、core `assets/litho.rs` |
| Chat / Ask 引擎 | native 与 ACP 双通道、prompt 组装、引用合并、用量记账 | `crates/terrain-agent/src/chat/`、`acp.rs`、`runtime.rs` |
| 工具与技能注入 | 工具 schema、preset skills、AGENTS.md 片段、agent tools 部署 | `crates/terrain-agent/src/tool_schema.rs`、`tools.rs`；core `agent_tools_deploy.rs`、`preset_skills.rs` |
| 环境集成 | env catalog 探测、plan、apply | `crates/terrain-core/src/assets/env/`、`src-tauri/src/env_catalog.rs` |
| CLI | 子命令分发与 JSON 输出 | `crates/terrain-cli/src/main.rs`、`commands/` |
| 桌面 IPC 与前端 | Tauri commands、Svelte 组件与 stores、i18n | `src-tauri/src/commands/`、`src/lib/components/`、`src/lib/stores/` |

## 核心流程

**1. 接入仓库并建立知识层（init）**
1. 校验仓库路径，写入全局 `~/.terrain/registry.json`，解析 `.terrain/` 布局。
2. `ProjectScanner` 走查文件树，按清单文件（`Cargo.toml`、`package.json`、`go.mod`、`pyproject.toml`、`pom.xml`、`build.gradle`）识别技术栈，产出 `index.md`；若存在 OpenAPI/Swagger 描述文件，额外产出 `interfaces/`。
3. 打包 repomix 源码包，写 `agent/repomix.md` 与 `agent/repomix.index.json`（本地，不入库）。
4. 生成 `agent/context.md` 宏观上下文（本文件），并按开发者 meta 合并结构化输入。
5. 建立 freshness 基线快照，写入 ledger 与 `.meta/freshness.json` 缓存。

**2. Ask 问答（native / ACP 双通道）**
1. 解析 project slug 与仓库路径，校验仓库有效性，创建或复用会话。
2. 组装 prompt：预载 `context.md` 概览；若上下文已陈旧（分数低于预载阈值），降级为"仅提示陈旧"而非注入过期结论。
3. 选择通道：native 走 `adk-model`（OpenAI / Ollama）并暴露工具集；ACP 走 `agent-client-protocol` 拉起外部 Agent 进程。
4. Agent 调用 `read-context` 取单节、`grep-pack` 定位、`read-pack-file` 读切片；无 Terrain 安装时 `rtk` / `codegraph` 降级为 `bunx` / `npx`。
5. 合并源码引用（citations），流式输出 chunk / 工具调用 / 阶段事件，记账 token 用量后落盘会话。

**3. Freshness 重算**
1. 取 Git 快照（是否 Git 仓库、HEAD、提交时间）。
2. 对每个资产层（repomix 包、agent 上下文、人类文档、index）读取各自 baseline，计算 `git_drift_since`。基线缺失或已被改写时**失败闭合**（记为不可测量，而非零漂移）。
3. `drift_factors` 归因（如 `index_missing_new_sources`、同步天数、未索引源文件）。
4. 按层打分并聚合出 overall score；结果回写 ledger 与缓存快照。
5. 分档决策：`FRESH_THRESHOLD = 80` 以上视为新鲜；低于预载阈值（`MACRO_PRELOAD_THRESHOLD`）时 Ask 拒绝预载宏观结论，并要求源码交叉验证。

**4. 环境集成（AGENTS.md / Skills / 工具链）**
1. 探测 Agent 环境（CLI 是否存在、版本、PATH、可写目录）。
2. 依据 `env-catalog` 生成计划：要写入的 AGENTS.md 片段、要安装的 skills、要部署的工具。
3. apply 阶段以标记块（`terrain:begin ... / terrain:end ...`）幂等改写 AGENTS.md。
4. 将捆绑的 `terrain` / `rtk` / `codegraph` 物化到 `~/.terrain/bin/`、`~/.terrain/tools/`：Unix 用符号链接，Windows 用指纹比对 + 原子替换 + 占用时降级。

## 技术选型

- **语言/工具链**：Rust 2024 edition，`rust-version = 1.94`，workspace 统一版本 1.1.2；`bun` 作包管理与脚本运行器；TypeScript + Vite。
- **异步与序列化**：`tokio`（full）、`serde` / `serde_json` / `serde_yaml`、`schemars`（工具 schema 生成）、`ts-rs`（IPC 类型导出）。
- **Agent 运行时**：`adk-core` / `adk-agent` / `adk-runner` / `adk-model` / `adk-acp` 2.2，`agent-client-protocol` 1.3（ACP 通道）。
- **打包引擎**：`repomix-core` 2.0（repomix 的 Rust 实现）。
- **桌面**：Tauri 2（`src-tauri`，capabilities 显式授权 + 托盘）。
- **前端**：Svelte 5 runes、Markdown + Mermaid 渲染、自建 i18n（en / zh-CN，脚本校验键位对齐）。
- **存储**：纯文件（JSON / JSONL / Markdown），无数据库、无 ORM。
- **分发**：`npm/packages/` 按平台分发 CLI 与二进制；`packages/` 存放 codegraph / rtk / terrain 包。
- **工程约束**：所有子进程统一走 `terrain_core::process::{command, async_command}`（Windows 自动 `CREATE_NO_WINDOW`），并由 `crates/terrain-core/tests/process_convention.rs` 扫描防止裸 `Command::new` 回归。

## 系统边界

| 边界 | 说明 | 信任/风险 |
|---|---|---|
| LLM Provider | 用户配置的 OpenAI 兼容端点或本地 Ollama，经 `adk-model` 调用 | 需 API Key；密钥仅存本地设置，写入 prompt 的内容视为外发 |
| 外部 ACP Agent | 通过 `agent-client-protocol` 拉起 Claude Code / Codex 等进程，`acp_spawn_command` 决定命令 | 子进程继承用户权限；prompt 由 Terrain 构造，仅授予 pack 读取工具 |
| Git 二进制 | 用于漂移测量与快照，非 libgit2 | 缺失时降级为同步天数估算（`not_git` 因子） |
| 仓库内容 | 不可信输入（提示注入、超大文件、二进制） | Agent **不读活仓库**，只读经过打包与裁剪的 repomix 包 |
| 全局状态目录 | `~/.terrain/`：`registry.json`、`bin/`、`tools/`、`sessions/`、`settings` | 跨项目共享；registry 仅存路径，不含知识正文 |
| 桌面 IPC | Tauri commands 为唯一前端通道，capabilities 控制权限面 | 前端不可直接访问文件系统 |
| 平台差异 | macOS / Windows 路径与可执行名、控制台窗口、符号链接 vs 拷贝 | Windows 打包版为 GUI 子系统，子进程必须无窗口 |
| 可选外部工具 | `rtk` / `codegraph`（`~/.terrain/bin/`），缺失时 `bunx` / `npx` 降级 | 执行失败直接跑原命令，不重试、不自动安装 |
| 无数据库 | 全部状态为文件；并发写依赖原子替换与锁文件 | 多实例并发需容忍"最后写入" |

## 代码映射索引

| 概念 | 位置 | 备注 |
|---|---|---|
| 知识路径布局 | `crates/terrain-core/src/paths.rs` | `.terrain/` 下各资产路径的唯一来源 |
| 项目注册表 | `crates/terrain-core/src/registry.rs`、`project.rs` | 全局 registry + 仓库路径解析 |
| 仓库扫描 | `crates/terrain-core/src/ingest/mod.rs` | 技术栈识别、`index.md`、OpenAPI |
| Repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | 生成 `repomix.md` / `repomix.index.json` |
| 包内检索 | `crates/terrain-core/src/assets/pack_read.rs`、`search.rs` | `grep-pack` / `read-pack-file` 实现 |
| Agent 上下文生成 | `crates/terrain-core/src/assets/agent_context.rs` | 本文件的产出逻辑 |
| 上下文分层 | `crates/terrain-core/src/assets/context_layers.rs` | macro / meso / micro 切分与阈值 |
| Freshness 计算 | `crates/terrain-core/src/freshness/compute.rs`、`scoring.rs` | 阈值常量、分层评分 |
| 漂移归因 | `crates/terrain-core/src/freshness/drift_factors.rs`、`git.rs` | 因子清单、Git 快照 |
| IPC / 状态类型 | `crates/terrain-core/src/schema/`、`ipc/` | 含 `freshness.rs`、`project.rs`、`chat.rs` |
| Chat 引擎 | `crates/terrain-agent/src/chat/` | `native.rs` / `acp.rs` / `prompt.rs` / `tracker.rs` |
| ACP 通道 | `crates/terrain-agent/src/acp.rs` | 外部 Agent 拉起与事件流 |
| Litho 文档流水线 | `crates/terrain-agent/src/litho.rs`、`litho_native.rs` | 预处理 → 研究 → 编排 → 输出 |
| CLI 入口 | `crates/terrain-cli/src/main.rs`、`commands/tools.rs` | `terrain tools` 子命令实现 |
| 类型导出 | `crates/terrain-ts-export/src/main.rs` → `src/lib/generated/` | `bun run gen:types` 生成，禁止手改 |