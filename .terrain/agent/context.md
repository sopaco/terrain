---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

**Terrain** 是面向 AI 编码助手时代的工程环境管理平台：为 Git 仓库自动构建"有地图、有路标、有规范"的知识领地。核心理念：**Terrain prepares the ground so agents don't have to guess where to stand**。

**消费者**：人类开发者（桌面应用/CLI）、外部 Coding Agent（`terrain tools` ACP 接口）、CI 流水线（`terrain refresh`）。

**关键约束**：知识资产存于 `{repo}/.terrain/` 随 Git 流转（非中心化 DB）；`terrain-core` 无 UI 依赖；双轨知识（`human/` 叙述 + `agent/` 压缩）；三层检索（Macro context → Meso 文档搜索 → Micro repomix grep/read）；`agent/context.md` ≤16 KiB；Agent 禁止直接读活仓库源码，以 repomix 包为准。

## 架构设计

### 容器与分层

| 层 | 容器 | 职责 |
|---|---|---|
| UI | `src/` Svelte 5 | DeepWiki/Litho/SDD/Env/Usage 面板 |
| Shell | `src-tauri/` | Tauri IPC 桥接（40+ invoke 命令） |
| Agent | `crates/terrain-agent/` | ChatEngine、Litho/SDD/上下文生成工作流 |
| Core | `crates/terrain-core/` | 扫描、资产、搜索、新鲜度、IPC 类型 |
| CLI | `crates/terrain-cli/` | 无头入口 + `terrain tools` ACP 工具 |
| 知识存储 | `.terrain/` | human/agent/knowledge/.meta/.litho-agent |

### 依赖方向

```
Svelte UI → Tauri commands → terrain-agent / terrain-core
terrain-cli → terrain-agent / terrain-core（共享领域逻辑）
terrain-agent → terrain-core → 文件系统 + repomix-core
terrain-agent → LLM Provider（Native）/ ACP 子进程（重任务）
```

### 核心模式

- **分层架构**：core → agent → cli/tauri → ui，CLI 与桌面共享业务逻辑
- **管道-过滤器**：Litho 四阶段、三层检索，中间产物可恢复
- **策略模式**：ChatEngine Native/ACP 双后端按任务路由
- **文件系统即存储**：`KnowledgePaths` 统一路径抽象

## 模块地图

| 模块 | 职责 | 主要路径 |
|---|---|---|
| terrain-core | 领域逻辑：路径、扫描、资产、搜索、新鲜度、IPC 类型 | `crates/terrain-core/src/` |
| terrain-agent | LLM 工作流：Ask/Litho/SDD/上下文生成 | `crates/terrain-agent/src/` |
| terrain-cli | CLI 命令树与 `tools` 子命令 | `crates/terrain-cli/src/` |
| Tauri 命令层 | 桌面 IPC 处理器 | `src-tauri/src/commands/` |
| Svelte 前端 | 面板 UI 与 `api.ts` 封装 | `src/lib/` |
| 知识资产层 | repomix 打包、context、Litho、env 集成 | `crates/terrain-core/src/assets/` |
| 源码扫描 | Git 元数据、OpenAPI、repomix 索引 | `crates/terrain-core/src/ingest/` |
| 新鲜度追踪 | 漂移检测、评分、ledger | `crates/terrain-core/src/freshness/` |
| preset_skills | Litho/SDD/Ask/Agent 技能定义 | `preset_skills/` |
| env-catalog | Skills/CLI/AGENTS.md 集成目录 | `env-catalog/` |
| ACP 补丁 | agent-client-protocol Windows 补丁 | `crates/agent-client-protocol-tokio-patched/` |
| npm 分发 | 跨平台 CLI shim 与二进制 | `npm/packages/` |

## 核心流程

### 1. 项目初始化（`terrain init`）

1. 用户注册仓库 → `initialize_project_cmd` / `run_project_initialization`
2. `ProjectScanner` 扫描 Git 元数据与 OpenAPI → `ScanReport`
3. repomix-core 打包源码 → `.terrain/agent/repomix.md`
4. Litho 四阶段 ACP 生成 → `.terrain/human/` 六份 C4 文档
5. Agent 上下文生成 → `.terrain/agent/context.md`（≤16 KiB）
6. 新鲜度 ledger 写入 → `.terrain/.meta/freshness.json`

### 2. DeepWiki 问答（Ask）

1. 用户提问 → `ask_knowledge` / `ChatEngine`
2. Macro：预载 `agent/context.md` 架构摘要
3. Meso：`search` 检索 human/knowledge 文档
4. Micro：`grep-pack` → `read-pack-file` 查 repomix 源码切片
5. Native LLM 或 ACP 子进程推理 + 工具调用
6. 流式返回 `AskStreamEvent` + `SourceCitation`

### 3. Litho 文档生成

1. `prepare_litho_generation` 规划阶段与技能注入
2. ACP 子进程执行四阶段（预处理→C4 研究→编排→输出）
3. 中间产物持久化 `.terrain/.litho-agent/`（支持中断恢复）
4. 输出六份标准 human 文档至 `.terrain/human/`

### 4. SDD 四阶段工作流

1. Requirements / TechDesign：Native LLM 生成 Markdown 产物
2. CodeGen / CodeReview：委托 ACP Agent 执行
3. 会话状态存 `.terrain/.meta/sdd-sessions/`
4. 每阶段产出可审查 Markdown，支持 `--session-id` 续接

## 技术选型

- **语言**：Rust（edition 2024 workspace）+ TypeScript + Svelte 5
- **桌面壳**：Tauri 2（`src-tauri/`）
- **前端构建**：Vite + Bun
- **异步运行时**：Tokio（全栈）
- **LLM 生态**：ADK（adk-model、adk-acp、adk-runner）
- **ACP 协议**：agent-client-protocol-tokio（本地 patch）
- **源码索引**：repomix-core（Rust 版 repomix）
- **类型导出**：ts-rs → `crates/*/bindings/` + `src/lib/generated/`
- **CLI 解析**：clap
- **跨平台分发**：npm optional deps（darwin-arm64 / win32-x64）
- **辅助工具**：codegraph（调用链）、rtk（检索工具）

## 系统边界

### 外部系统

| 边界 | 交互方式 | 说明 |
|---|---|---|
| Git 仓库 | 文件系统读取 | 源码输入、分支元数据、新鲜度基准 |
| LLM Provider | HTTP API | OpenAI / Ollama / LM Studio |
| ACP Agent | 子进程 stdio | OpenCode / Cursor CLI 等 |
| 文件系统 | 读写 `.terrain/` | 知识唯一持久化层 |
| `~/.terrain/registry.json` | 本地注册表 | 项目 slug ↔ repo 指针（非知识） |
| `~/.terrain/settings.json` | 用户配置 | ModelSettings（LLM/ACP profiles） |

### 对外接口

| 接口 | 入口 | 消费者 |
|---|---|---|
| CLI | `terrain` 二进制 | 开发者、CI、脚本 |
| ACP tools | `terrain tools *`（JSON stdout） | 外部 Coding Agent |
| Tauri IPC | `invoke("*_cmd")` | Svelte 桌面 UI |
| AGENTS.md 片段 | `env-catalog/agents-md/` | 外部 Agent 环境集成 |

### 信任边界

- Agent **不得**直接读活仓库；以 `agent/repomix.md` 为源码权威
- 新鲜度 `<50` 时宏观 context 不可信，以 repomix 为准
- 矛盾优先级：**repomix > codegraph > context.md > human/**

## 代码映射索引

| 概念 | 位置 | 备注 |
|---|---|---|
| KnowledgePaths | `crates/terrain-core/src/paths.rs` | 所有 `.terrain/` 路径根抽象 |
| ProjectScanner | `crates/terrain-core/src/ingest/` | 扫描入口 |
| repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | 生成 `agent/repomix.md` |
| 三层检索定义 | `crates/terrain-core/src/assets/context_layers.rs` | Macro/Meso/Micro + 16KiB 限制 |
| ChatEngine | `crates/terrain-agent/src/chat/mod.rs` | Native/ACP 双后端路由 |
| 项目初始化 | `crates/terrain-agent/src/workflows/init.rs` | scan→Litho→context 流水线 |
| Ask 工作流 | `crates/terrain-agent/src/workflows/ask.rs` | DeepWiki 入口 |
| Litho 生成 | `crates/terrain-agent/src/litho.rs` | 四阶段 ACP 编排 |
| SDD 工作流 | `crates/terrain-agent/src/workflows/sdd.rs` | 四阶段 SDD |
| Agent 上下文生成 | `crates/terrain-agent/src/agent_context.rs` | 产出 context.md |
| IPC 类型 | `crates/terrain-core/src/ipc/` | 跨边界共享类型 |
| Tauri 命令注册 | `src-tauri/src/lib.rs` | invoke 命令表 |
| 前端 API 封装 | `src/lib/api.ts` | invoke 调用层 |
| CLI 命令树 | `crates/terrain-cli/src/cli.rs` | clap 定义 |
| ACP tools | `crates/terrain-cli/src/commands/tools.rs` | grep-pack/read-pack-file 等 |
| 环境集成 | `crates/terrain-core/src/assets/env/` | Skills/AGENTS.md 部署 |