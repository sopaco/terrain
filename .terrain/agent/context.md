---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手时代的**工程环境管理平台**：注册 Git 仓库后自动扫描代码结构、生成 C4 架构文档（Litho）、维护双轨知识资产（人类可读 `human/` + 机器友好 `agent/`），并通过 DeepWiki 问答与 SDD 四阶段工作流，让人类开发者与外部 Coding Agent 共享同一套知识契约。知识存放在仓库内 `.terrain/`，随 Git 分支流转（"知识跟着代码走"），非中心化数据库。主要消费者：桌面应用（Tauri+Svelte）、`terrain` CLI、外部 Coding Agent（经 AGENTS.md / Skills / ACP 接入）。核心约束：Rust 为 IPC 类型唯一真源（ts-rs 生成前端类型，禁手改生成物）；repomix 索引与 context.md 为生成物，不手工合并冲突，靠 freshness 表达滞后。

## 架构设计

| 容器/层 | 路径 | 职责 |
|---|---|---|
| 前端 Svelte 5 + TS | `src/` | 面板式 UI：项目总览、Ask、DeepWiki、SDD、Env 集成、Freshness、Usage、源码查看 |
| 桌面壳 Tauri 2 | `src-tauri/` | 窗口/托盘、IPC command 注册、sidecar 打包（rtk、terrain-cli、codegraph、preset_skills） |
| 领域核心 | `crates/terrain-core/` | 纯领域逻辑（无 Agent 依赖）：扫描、知识资产、freshness、会话、检索、registry、env、usage、IPC 类型 |
| Agent 编排 | `crates/terrain-agent/` | ChatEngine（Native LLM + ACP 双后端）、Litho、SDD、workflows、上下文生成、工具集 |
| CLI | `crates/terrain-cli/` | 无头场景：ask/scan/init/sdd/env/settings/usage/source 等命令 |
| 类型导出 | `crates/terrain-ts-export/` | ts-rs 汇总导出 → `src/lib/generated/` |
| 配套产物 | `preset_skills/`、`env-catalog/`、`npm/`、`packages/` | 随包分发的 Skills、Agent 环境目录、CLI npm 发行包、捆绑工具占位 |

依赖方向：`src-tauri` → `terrain-agent` → `terrain-core`；`terrain-cli` → `terrain-core`（ask/sdd 复用 `terrain-agent`）。前端仅经 `src-tauri` IPC 调用 Rust。`terrain-ts-export` 由 `bun run gen:types` 触发。

## 模块地图

| 模块 | 职责 | 主要路径 |
|---|---|---|
| ingest | `ProjectScanner` 扫描（Git 元数据/OpenAPI 导入）+ repomix 打包 `agent/repomix.md` | `crates/terrain-core/src/ingest/` |
| assets | agent context 生成/读取、repomix grep/read、Litho 人类文档、SDD 产物、meta 输入、Ask 分层上下文 | `crates/terrain-core/src/assets/` |
| freshness | 知识保鲜评分、git 快照/ledger、codegraph drift 交叉验证 | `crates/terrain-core/src/freshness/` |
| sessions | Ask/SDD 会话创建、持久化、active 状态 | `crates/terrain-core/src/sessions/` |
| search / doc / human | 三层检索、Markdown 解析/渲染、human 文档计数读取 | `crates/terrain-core/src/{search.rs,doc.rs,human.rs}` |
| registry / project / paths / repo | 项目登记、repo 校验、路径解析（portable）、overview 汇总 | `crates/terrain-core/src/{registry.rs,project.rs,paths.rs,repo.rs}` |
| settings / schema / ts_ipc | 模型与 ACP 设置；IPC/状态类型（ts-export）；类型生成宏 | `crates/terrain-core/src/{settings.rs,schema/,ts_ipc.rs}` |
| integrations + env | 环境探测/规划/应用、Agent 工具链部署、bundled tools、usage 探测、预设 Skills 分发 | `crates/terrain-core/src/{integrations/,assets/env/,usage.rs}` |
| chat | `ChatEngine`：native.rs（OpenAI 兼容 LLM）+ acp.rs（ACP 子进程）+ 工具执行 + token 追踪 | `crates/terrain-agent/src/chat/` |
| workflows | ask / init / quick_refresh / sdd 高层流程 | `crates/terrain-agent/src/workflows/` |
| litho / sdd | C4 四阶段文档与 SDD 四阶段编排（ACP 委托 + 重试） | `crates/terrain-agent/src/{litho.rs,sdd.rs}` |
| agent 上下文/资产 | `AgentContextGenerator` trait、ask 前资产保鲜、工具 schema | `crates/terrain-agent/src/{context_generator.rs,agent_assets.rs,agent_context.rs,tools.rs}` |
| 桌面 IPC | Tauri command 组（assets/env/knowledge/project/sessions/settings/usage/workflows） | `src-tauri/src/commands/` |
| 前端 | 面板组件、stores、api.ts IPC 封装、markdown/mermaid/源码高亮 | `src/lib/` |

## 核心流程

1. **项目注册与扫描**：`register_project`（registry）→ 校验 repo → `ProjectScanner` 采集 Git 元数据 + 导入 OpenAPI → repomix 打包 `agent/repomix.md` → 初始化 freshness ledger → 生成 `agent/context.md`（macro 层）→ 可触发 Litho。
2. **Ask / DeepWiki 问答（三层检索）**：创建/恢复 Ask session → ChatEngine 校验 agent 资产与 HEAD 同步（落后则先 quick refresh）→ Macro 预载 context.md → Meso 检索 `human/`+`knowledge/` → Micro grep/read repomix 包 → Native LLM 或 ACP 子进程生成 → 流式事件 + 来源引用（citations）→ 会话落盘。
3. **Litho C4 文档生成**：预处理（收集 repo/meta）→ C4 研究 → 编排 → 输出 6 份人类文档（含 Mermaid）；中间产物在 `.terrain/.litho-agent/`，支持中断恢复与重试。
4. **SDD 四阶段开发**：需求 → 技术设计 → 代码生成 → 代码审查，每阶段产出可审查 Markdown；轻量文档阶段走 Native LLM，代码生成委托 ACP Agent。
5. **环境集成**：probe（检测 skills/工具链/AGENTS.md 片段）→ plan（`EnvPlan` 步骤）→ apply（部署到仓库并写 AGENTS.md 片段）→ 状态缓存失效。

## 技术选型

- Rust workspace（edition 2024, rust-version 1.94；成员：terrain-core / terrain-agent / terrain-cli / terrain-ts-export / src-tauri）+ tokio + tracing
- ACP 协议栈：adk-acp / adk-agent / adk-core / adk-model（ollama、openai）/ adk-runner / adk-session / adk-tool；`agent-client-protocol` 0.11 打补丁版（`crates/agent-client-protocol-tokio-patched`，经 `[patch.crates-io]` 接入）
- Tauri 2：`externalBin` 打包 rtk、terrain-cli sidecar；resources 打包 codegraph 与 preset_skills
- 前端：Svelte 5 + TypeScript + Vite 8 + Tailwind 4；marked、mermaid、highlight.js、html2canvas、@lucide/svelte
- ts-rs 10：`bun run gen:types` 生成 `src/lib/generated/`（禁手改）
- repomix-core 2.0：源码打包成 grep 友好索引包
- 捆绑工具：rtk（token 压缩 shell）、codegraph（符号关系）、terrain-cli，多平台（darwin-arm64 / win32-x64）
- npm 发行：`npm/packages/cli` + 平台 shim 包（`npm/scripts/write-shims.mjs`）

## 系统边界

- **外部 LLM**：OpenAI 兼容 HTTP 端点（LM Studio / Ollama / OpenAI），经 `chat/native.rs` NativeBackend（adk-model）
- **外部 ACP Agent**：子进程（如 opencode）按会话 spawn，提供 tool 调用与流式事件；`AcpSettings` 配置 binary/args/执行模式（AcpNative / AcpDelegate 等）
- **仓库文件系统**：`.terrain/`（知识资产）、`.terrain/.litho-agent/`（Litho 研究产物）、`.codegraph/`（符号索引 DB）；`.terrain/.gitignore` 声明不入库的本地衍生物
- **本地用户目录**：`~/.terrain/registry.json`（仅仓库指针）、`~/.terrain/bin/`（工具约定路径）、用户 preset skills
- **随包资源**：`preset_skills/`（litho/sdd/ask/architecture/context）、`env-catalog/`（catalog + skills + AGENTS.md 片段模板）
- **信任边界**：ACP Agent 在仓库内执行、可读写文件；模型设置与 API 密钥存本地；Tauri 配置 CSP=null；sidecar 二进制随发行包分发
- **不依赖中心化知识库**：知识随 Git 分支流动，三方合并只针对 `knowledge/`，生成物禁止自动合并

## 代码映射索引

| 概念 | 位置 | 备注 |
|---|---|---|
| ProjectScanner / ScanReport / repomix 打包 | `crates/terrain-core/src/ingest/` | 含 openapi.rs、git.rs |
| agent/context.md 生成与读取 | `crates/terrain-core/src/assets/agent_context.rs` | write/read/split/extract |
| repomix 包读写与 grep | `crates/terrain-core/src/assets/{repomix.rs,pack_read.rs,query.rs}` | pack-meta/read-pack-file 数据源 |
| 知识保鲜评分 | `crates/terrain-core/src/freshness/` | compute/scoring/git/codegraph/ledger/drift_factors |
| 三层检索 | `crates/terrain-core/src/{search.rs,assets/context_layers.rs,assets/ask.rs}` | KnowledgeSearch |
| ChatEngine（双后端） | `crates/terrain-agent/src/{runtime.rs,chat/}` | native.rs / acp.rs / prompt.rs / tracker.rs |
| Ask 工作流 | `crates/terrain-agent/src/workflows/ask.rs` | 含 search fallback |
| Litho 四阶段 | `crates/terrain-agent/src/litho.rs` | run_litho_generation / 重试 |
| SDD 四阶段 | `crates/terrain-agent/src/sdd.rs`、`workflows/sdd.rs` | 阶段产物输出 |
| ACP 子进程与兼容层 | `crates/terrain-agent/src/{acp.rs,chat/acp.rs,compat_tool.rs}` | 会话、工具适配 |
| IPC 类型与 ts-rs | `crates/terrain-core/src/schema/`、`crates/terrain-agent/src/chat/types.rs` | `ts-export` feature |
| Tauri commands | `src-tauri/src/commands/` | assets/env/knowledge/project/sessions/settings/usage/workflows |
| 前端 IPC 封装与状态 | `src/lib/{api.ts,stores/,types.ts}` | invoke 封装、runes stores |
| Env 集成/工具链部署 | `crates/terrain-core/src/{integrations/,assets/env/,agent_tools_deploy.rs,bundled_tools.rs}` | probe/plan/apply |
| 预设 Skills | `preset_skills/`、`crates/terrain-core/src/preset_skills.rs` | 打包进 app 资源 |
| CLI 命令 | `crates/terrain-cli/src/commands/` | ask/assets/env/init/knowledge/project/sdd/settings/source/tools/usage |