---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是一个**标准化、AI 友好的工程环境管理工具**：指向一个 Git 仓库，自动产出三层知识资产（人类可读的 `human/` C4 文档、Agent 消费的 `agent/context.md` 与源码索引 `agent/repomix.md`、业务术语 `knowledge/`），并一键部署 Agent 工具链（CodeGraph、RTK、预设 Skills、受管理的 `AGENTS.md` 片段）。隐喻：知识是地图，工具是道路，流程/约定是路标。提供桌面应用（Tauri GUI）与 CLI 双形态，外部编码 Agent（Claude Code、Codex、OpenCode、ACP）通过 `terrain tools` JSON 接口消费同一份知识。核心约束：scan/pack/search/freshness/env 全部离线纯 Rust，不依赖 LLM；知识资产随 Git 分支流转（`.terrain/` 入库）；采用增量更新 + 新鲜度评分追踪与代码的漂移。

## 架构设计

```
┌──────────┬───────────────┬───────────────┐
│ 桌面应用   │     CLI        │   外部编码 Agent │
│ src/     │ terrain       │  terrain tools  │
│(Svelte 5)│ 15 子命令      │ (JSON stdout)  │
└─────┬────┴───────┬───────┴───────┬───────┘
      │ Tauri IPC  │               │
      ▼            ▼               ▼
┌──────────────────────────────────────────┐
│          terrain-agent (编排层, 需 LLM)       │
│  Ask/DeepWiki  · Litho 知识生成 · SDD · context │
│  ACP 子进程 · 原生 LLM · 会话 · 工具调用 · 节流      │
└───────────────────┬──────────────────────┘
                    ▼
┌──────────────────────────────────────────┐
│          terrain-core (离线引擎)             │
│ scan · pack · search · freshness · env · │
│ registry/repo · schema · ipc · sessions  │
└───┬─────────┬──────────┬──────────┬──────┘
    ▼         ▼          ▼          ▼
 Git 仓库    ~/.terrain/   LLM/ACP    Repomix·
 .terrain/  registry.json  (HTTP)    CodeGraph
```

- **terrain-core**：无 LLM 依赖的离线内核，负责一切文件系统/Git/打包/评分/Env 部署逻辑，是唯一真源（schema、IPC 载荷）。
- **terrain-agent**：编排层，面向 LLM 能力工作流；通过 trait + ACP 设置决定走原生 LLM 还是 ACP 子进程。
- **terrain-ts-export**：编译期用 ts-rs 把 Rust IPC 类型导出为 TypeScript，前端禁止手改 `src/lib/generated/`。
- **src-tauri**：Tauri 2 外壳，将 core/agent 能力以命令形式暴露给前端；含 tray、能力(ACL)、打包脚本。
- **npm/packages + npm/scripts**：跨平台二进制分发（`@terrain-ai/cli`、`@terrain-ai/rtk`，darwin-arm64 / win32-x64）。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| 知识资产(assets) | scan/pack/context/litho/incremental 生成与状态 | `crates/terrain-core/src/assets/` |
| 打包/读取 | repomix 打包、pack 读取、grep | `assets/repomix.rs`、`assets/pack_read.rs` |
| 新鲜度 | Git drift 计算、评分、CodeGraph drift | `crates/terrain-core/src/freshness/` |
| 仓库扫描 | `ProjectScanner`、`ScanReport`、`AgentPackSummary` | `crates/terrain-core/src/ingest/` |
| Env 集成 | 工具链部署/状态/计划、内置工具、usage 探测 | `crates/terrain-core/src/integrations/` |
| Schema/IPC | 所有 IPC/状态类型（ts-rs 导出源） | `crates/terrain-core/src/schema/`、`ipc/` |
| 注册表/项目 | 项目登记、概览、仓库路径 | `registry.rs`、`project.rs`、`repo.rs` |
| 会话 | Ask/SDD 会话持久化与恢复 | `crates/terrain-core/src/sessions/` |
| Agent 运行时 | `Runtime`/`ChatEngine` 生命周期、模型配置 | `crates/terrain-agent/src/runtime.rs`、`chat/` |
| 工作流 | ask/init/sdd/quick_refresh 编排 | `crates/terrain-agent/src/workflows/` |
| ACP 桥 | ACP 子进程 spawn/配置/可用性 | `crates/terrain-agent/src/acp.rs`、`chat/acp.rs` |
| 工具面 | 知识查询工具（list/read-context/search/grep/read-pack） | `crates/terrain-agent/src/tools.rs`、`compat_tool.rs` |
| CLI | 15 个子命令（assets/init/ask/env/sdd/tools…） | `crates/terrain-cli/src/commands/` |
| Tauri 命令层 | 暴露 core/agent 给 GUI | `src-tauri/src/commands/` |
| 前端 UI | 面板/阅读器/问答/Env/SDD/用量/设置 | `src/App.svelte`、`src/lib/components/`、`stores/` |

## 核心流程

1. **项目初始化与知识生成**：注册仓库（写 `~/.terrain/registry.json`）→ scan 生成项目索引 `.terrain/index.md` → 离线 pack 生成 `agent/repomix.md` → LLM/ACP 生成 `agent/context.md` → ACP 多阶段研究生成 `human/` C4 文档（带 `.litho-agent/` 检查点）→ 追踪 freshness。增量模式下按 Git HEAD 变更只重生成受影响资产。
2. **Ask / DeepWiki 问答**：加载问答会话 → 组装三层知识 prompt（宏观 `read-context`、中观 `search`/`read-doc`、微观 `grep-pack`→`read-pack-file`）→ 引入引文与来源切片 → 流式返回 `AskStreamEvent`，前端 Thinking/ToolCall/答案三窗口渲染；来源冲突时按 repomix > CodeGraph > context > human 的优先级降权。
3. **Agent 环境集成**：`env status` 检查工具/Skills/`AGENTS.md` 片段 → `env plan` 预览变更 → `env apply` 按依赖顺序把 CodeGraph、RTK、`terrain` CLI 与预设 Skills 部署到 `~/.terrain/bin` 与用户 Skills 目录，并写入带知识优先约定的 `AGENTS.md` 模板片段。
4. **SDD 工作流**：需求（原生 LLM）→ 技术设计（原生 LLM）→ 代码生成（ACP Agent 隔离执行并落盘）→ 代码复审；四阶段产物为可审阅的 Markdown，输出与检查点到 `~/.terrain/sdd/{project}/sessions/`。
5. **增量刷新 + 新鲜度**：`quick_refresh` 提交后低成本刷新（跳过文档重生成）→ 计算各资产相对 Git baseline 的 drift（commits / changed_files / dirty / days_since_sync）→ 写入 `.terrain/.meta/freshness.json`，低于阈值时 Agent 自动降权宏观上下文。

## 技术选型

- **Rust workspace**（MSRV 1.94）：terrain-core / terrain-agent / terrain-cli / terrain-ts-export + src-tauri。
- **Tauri 2** 桌面外壳：能力(ACL)、托盘、跨平台打包（macOS Apple Silicon / Windows x64）。
- **前端**：Svelte 5（runes 状态 `*.svelte.ts`）、TypeScript、Vite、Bun；Markdown + Mermaid 渲染、DeepWiki 面板。
- **双语言 i18n**：`en` / `zh-CN` 全量键位 + parity 检查脚本。
- **类型契约**：ts-rs 生成 TS（`bun run gen:types`），`Option<T>` → `T | null`。
- **知识打包**：repomix-rs（Rust 锈化）；CodeGraph 用于符号调用方/影响分析。
- **LLM**：OpenAI 兼容 / Ollama / LM Studio；ACP 模式经通用 Agent 子进程执行重型工作。
- **离线优先**：scan/pack/search/freshness/env 全程离线、单二进制、无数据库、不调 LLM。
- **分发**：npm 平台包（@terrain-ai/cli、@terrain-ai/rtk，darwin-arm64/win32-x64）+ GitHub Releases 安装包。

## 系统边界

- **Git 仓库（读写）**：scan/pack/freshness 读取 repo，`.terrain/` 资产写入仓库并随分支流转；`.gitignore`/`.gitattributes` 控制哪些资产入库、哪些本地重建。
- **`~/.terrain/registry.json`**：仅项目指针（路径 + slug），不含知识正文。
- **用户目录部署**：`~/.terrain/bin/{terrain,rtk,codegraph}`、`~/.terrain/skills/`、`~/.terrain/sdd/`（用户级不存在版本控制，不写入仓库）。
- **外部 Agent 子进程（ACP）**：`AcpSettings`（binary/args/env）spawn opencode/claude/codex 等；本轮以 ACP 模式执行时 Agent 仅可通过 `terrain tools` CLI（JSON stdout）访问知识。
- **LLM Provider（HTTP）**：各 provider 默认 token 端点、base_url、密钥等集中在 `settings.rs` 的 provider profile。
- **第三方二进制**：repomix CLI（打包）、codegraph CLI（drift 与调用分析）、rtk（输出压缩）。
- **Tauri IPC 信任边界**：Rust 为唯一真源，前端经生成类型访问；`src-tauri/capabilities/` 声明命令 ACL。
- **重写/冲突优先序**：repomix 源码 > CodeGraph > agent/context.md > human 文档；`freshness < 50` 时降权宏观层。

## 代码映射索引

| 概念 | 位置 | 说明 |
|------|------|------|
| 知识工厂(核心) | `crates/terrain-core/src/assets/` | scan/pack/context/litho/incremental 状态机 |
| Repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | 调用 repomix 生成 `agent/repomix.md` |
| 新鲜度评分/漂移 | `crates/terrain-core/src/freshness/` | scoring + drift_factors + codegraph + ledger |
| 仓库扫描器 | `crates/terrain-core/src/ingest/` | ProjectScanner / ScanReport |
| Env 部署与状态 | `crates/terrain-core/src/integrations/` | plan/apply/env catalog/usage 探测 |
| IPC/Schema 类型 | `crates/terrain-core/src/schema/`、`ipc/` | ts-rs 导出源（`src/lib/generated/` 为产物） |
| 会话持久化 | `crates/terrain-core/src/sessions/` | Ask/SDD 会话存取 |
| Agent 运行时/引擎 | `crates/terrain-agent/src/runtime.rs`、`chat/` | ChatEngine、prompt、tracker、节流 |
| ACP 子进程 | `crates/terrain-agent/src/acp.rs`、`chat/acp.rs` | ACP 配置/spawn/纯 ACP 判定 |
| 工作流编排 | `crates/terrain-agent/src/workflows/` | ask / init / sdd / quick_refresh |
| 知识查询工具面 | `crates/terrain-agent/src/tools.rs` | read-context / search / grep-pack 等工具 |
| CLI 子命令 | `crates/terrain-cli/src/commands/` | assets/env/ask/sdd/tools/knowledge… |
| Tauri 命令层 | `src-tauri/src/commands/` | 暴露 core/agent 能力、payloads |
| 前端状态与 UI | `src/lib/stores/`、`src/lib/components/` | 项目/会话/状态/用量 store；各功能面板 |
| 预设 Skills | `preset_skills/` | terrain-ask / agent-architecture / sdd / litho |
| 分发与目录 | `npm/`、`packages/`、`env-catalog/` | 平台二进制、shim 脚本、env 清单 |