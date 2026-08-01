---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 架构设计

| 容器 | 角色 | 关键依赖 |
|------|------|----------|
| 桌面 App（src-tauri + src/） | 主界面与 IPC 宿主 | Tauri v2、Svelte 5、Vite、Bun |
| terrain-core | 领域核心：schema/assets/freshness/ingest | serde、tokio、ts-rs、repomix-core |
| terrain-agent | LLM 编排：ChatEngine、workflows、ACP/Native | adk-*、agent-client-protocol、opencode |
| terrain-cli | 命令行入口 | clap 风格子命令 |
| terrain-ts-export | 导出 Rust 类型 → TS bindings | ts-rs |
| agent-client-protocol-tokio-patched | ACP 客户端 fork（crates-io patch） | adk-acp |
| 外部 LLM/Agent | Native LLM / ACP 子进程 | OpenAI/Ollama / opencode |

分层：Svelte UI（`src/lib`）→ Tauri IPC commands（`src-tauri/src/commands/`）→ terrain-core / terrain-agent 业务 → 文件系统 `.terrain/`、外部 LLM、ACP 子进程、CLI 工具链。IPC 类型唯一真源在 Rust（`crates/terrain-core/src/schema/` 与 `crates/terrain-agent/`），经 `gen:types` 生成 `src/lib/generated/`。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| Tauri 壳 | 命令注册、tray、bundled tools/skills 装配 | `src-tauri/src/` |
| 前端 UI | 面板、Ask/SDD/Env/Litho 界面、stores | `src/lib/components/`、`src/lib/stores/` |
| 前端 IPC 封装 | invoke 封装、流式 Channel、类型 | `src/lib/api.ts`、`src/lib/types.ts` |
| assets | agent context / repomix pack / context layers / 文档生成 | `crates/terrain-core/src/assets/` |
| freshness | 知识保鲜评分、git drift、codegraph 交叉验证 | `crates/terrain-core/src/freshness/` |
| ingest | git 元数据、OpenAPI 规范采集 | `crates/terrain-core/src/ingest/` |
| schema/ipc | IPC 类型、命令路由、会话 | `crates/terrain-core/src/schema/`、`ipc/` |
| 环境集成 | Skills/AGENTS.md/工具链探测与部署、usage | `crates/terrain-core/src/integrations/`、`assets/env/` |
| ChatEngine | Native LLM 与 ACP 双后端聊天 | `crates/terrain-agent/src/chat/` |
| workflows | ask / init / sdd / quick_refresh 工作流 | `crates/terrain-agent/src/workflows/` |
| Agent 生成 | agent context、litho 编排、ACP 会话 | `crates/terrain-agent/src/{context_generator,litho,acp}.rs` |
| CLI | 子命令入口与 util | `crates/terrain-cli/src/commands/` |
| TS 导出 | 根类型汇总导出 | `crates/terrain-ts-export/src/main.rs` |

## 核心流程

**① 项目注册与初始化**：`init` → 采集 git 元数据/OpenAPI → 扫描写入 `.terrain/`（索引、knowledge 库）→ repomix 打包 `agent/repomix.md` → 生成 `agent/context.md` → 可选触发 Litho 文档。进度经 ProgressEvent 流式回传 UI/CLI。

**② DeepWiki Ask（三层检索）**：Macro 预载 `agent/context.md` 概览 → Meso 检索 `human/` + `knowledge/` 文档并做来源引用 → Micro 通过 `grep-pack` + `read-pack-file` 读 repomix 源码切片；ChatEngine 按配置走 Native LLM 或 ACP 子进程，回答与工具调用经 Channel 流式推送，token/工具调用记录回写会话。

**③ Litho C4 文档生成**：plan → 四阶段流水线（预处理 → C4 研究 → 编排 → 输出）产出六份标准人类文档（含 Mermaid）；中间产物持久化 `.terrain/.litho-agent/`，支持中断恢复（resume/forceRefresh）。

**④ SDD 四阶段开发**：需求 → 技术设计 → 代码生成 → 代码审查，每阶段产出可审查的 Markdown；轻量阶段走 Native LLM，代码生成委托 ACP Agent；会话与产物按 session 归档（`.terrain/sdd/`）。

**⑤ 知识保鲜（freshness）**：基于 Git HEAD 与 dirty-state 对知识资产打分（freshness_score），`<70` 需交叉验证、`<50` 宏观不可信；`codegraph-drift` 用 git 独立检测 codegraph 索引过期。

## 技术选型

- **后端**：Rust（edition 2024，rust-version 1.94，tokio full、anyhow、serde/serde_yaml、tracing）
- **桌面**：Tauri v2（src-tauri）、多窗口（主窗口 + UsageWindow）、tray
- **前端**：Svelte 5 runes（`.svelte.ts` stores）、TypeScript、Vite、Bun（bun.lock）、marked + 自定义 markdown 渲染/语法高亮/mermaid
- **LLM**：adk-* 系列（adk-model 支持 OpenAI/Ollama、adk-runner/session/tool）、agent-client-protocol 0.11（unstable_session_usage）、opencode ACP 子进程
- **Agent 索引**：repomix-core 2.0（打包 `agent/repomix.md`）
- **类型契约**：ts-rs 生成 TS bindings（`terrain-ts-export`）
- **工具链**：`~/.terrain/bin/`（rtk / codegraph / terrain），npm platform 包分发（darwin-arm64 / win32-x64），Bun/NPM shim
- **代码质量**：`bun run check`（lint+typecheck）、`bun run gen:types`

## 系统边界

| 边界 | 方向 | 信任/约束 |
|------|------|-----------|
| Git 仓库 | 读 + 受控写（`.terrain/` 内） | 由 git_policy 约束；不污染用户源码 |
| LLM 提供商 | 出站 API（OpenAI/Ollama 等，adk-model） | 模型配置来自 settings，可切换 Native/ACP |
| ACP Agent（opencode） | 子进程双向 IPC | `agent-execution` 决定执行后端；`check_acp`/`check_opencode` 探测；会话含 token 计费与工具调用记录 |
| repomix/codegraph/rtk 工具 | 子进程调用 | bundled 二进制 + npm platform 包；codegraph-drift 校验索引过期 |
| `.terrain/` 知识目录 | 生成 + 写入 | `agent/`、`generated/` 为生成物；git 合并策略见 `.gitignore/.gitattributes` |
| OpenAPI 导入 | 入站规范采集 | ingest/openapi.rs |
| 环境部署 | 写 `~/.terrain/bin/`、仓库 AGENTS.md/Skills | 需 Env 集成审批（plan → apply），回滚依赖 catalog |
| CLI（terrain） | 独立进程消费同一核心 | 命令集合见 cli.rs（scan/init/refresh/search/ask/sdd/env/tools/assets/usage） |

信任边界强调：ACP 子进程执行任意 Agent 代码；Env 集成修改工具链与仓库文件；`terrain-ts-export` 产出是唯一可信的 TS 类型来源（禁止手改 `src/lib/generated/`）。

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| IPC 命令注册 | `src-tauri/src/commands/` | assets/env/knowledge/project/sessions/settings/usage/workflows.rs |
| 前端 invoke 封装 | `src/lib/api.ts` | 覆盖全部 IPC 命令 + 流式 Channel |
| IPC 类型（Rust 真源） | `crates/terrain-core/src/schema/`、`ipc/` | 加 ts-export 注解 |
| Agent/Chat IPC 类型 | `crates/terrain-agent/` | bindings/ 生成产物 |
| TS 类型汇总导出 | `crates/terrain-ts-export/src/main.rs` | `gen:types` 入口 |
| ChatEngine 双后端 | `crates/terrain-agent/src/chat/{native,acp}.rs` | 模式由 AcpSettings.agent_execution 决定 |
| 工作流 | `crates/terrain-agent/src/workflows/{ask,init,sdd,quick_refresh}.rs` | |
| Litho 编排 | `crates/terrain-agent/src/litho.rs` + `assets/litho.rs` | 四阶段 |
| Agent context 生成 | `crates/terrain-core/src/assets/agent_context.rs`、`crates/terrain-agent/src/context_generator.rs` | |
| repomix pack | `crates/terrain-core/src/assets/repomix.rs`、`pack_read.rs` | |
| freshness 评分 | `crates/terrain-core/src/freshness/` | compute/git/codegraph/drift_factors |
| Env 集成 | `crates/terrain-core/src/assets/env/`、`integrations/mod.rs` | status/plan/apply |
| ACP 客户端 fork | `crates/agent-client-protocol-tokio-patched/src/acp_agent.rs` | crates-io patch |
| CLI 子命令 | `crates/terrain-cli/src/commands/` | 15 组子命令 |
| Preset Skills | `preset_skills/`、`src-tauri/src/preset_skills.rs` | Litho/SDD/Ask/Context 技能注入 |
| 前端 stores | `src/lib/stores/` | chat/project/status/usageDisplay/readerLayout |