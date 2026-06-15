---
type: agent_context
project: mind-mesh
title: Agent Architecture Context
source: /Users/bjsttlp485/Workspace/SAW/mind-mesh
---

## 项目概览

MindMesh 是一个面向 AI 编码助手的工程环境管理平台。它能自动扫描任意代码仓库，分析项目结构，生成人类可读的 C4 模型架构文档（Mermaid）及结构化的 AI Agent 上下文知识资产（如 `agent/context.md`）。系统通过扫描仓库源码，调用 LLM 进行架构分析，产出项目概览、架构设计、模块地图、核心流程、技术选型和系统边界等知识，同时维护环境集成、技能索引和工具调用追踪，支持 DeepWiki 模式下的即时问答与 SDD 生成。

## 架构设计

### 系统容器* **用户界面层 (User Interface)

**: * **CLI**: `crates/mind-mesh-cli`，命令行入口，负责初始化、扫描和生成知识资产。 * **桌面应用 (Tauri)**: `src-tauri` (Rust 后端) + `src/` (Svelte 前端)，提供可视化界面。* **核心处理层 (Core Processing)**: * **Core Library**: `crates/mind-mesh-core`，业务逻辑核心，负责扫描、注册、打包（Repomix）、文档生成、搜索、问答推理及知识持久化。 * **Agent Layer**: `crates/mind-mesh-agent`，AI Agent 专用适配层，负责 ACP 协议通信、Prompt 组装、上下文生成及 Agent 工具执行。* **基础设施层 (Infrastructure)**: * **LLM Provider**: 外部大语言模型接口（需用户配置 Key）。 * **Git Repository**: 被扫描的代码仓库作为数据源。 * **Local Storage**: `.mind-mesh/` 目录存储生成的知识资产。 * **File System**: 用于读取源码和写入产物。

### 架构依赖* **内部依赖**: `crates/mind-mesh-core` 被 `crates/mind-mesh-cli` 和 `crates/mind-mesh-agent` 调用。* **外部依赖**: `tauri` (桌面框架)

, `svelte` (前端框架), `Rust` (系统语言), `LLM API` (推理引擎)。

### 主要数据流

```mermaidgraph
 TD Input[Git 源码目录] --> |fs| RepoRead
    RepoRead --> |fs| PackRead
    PackRead --> |LLM| CoreLLM
    CoreLLM --> |Doc| LithoOutput[人类文档]
    CoreLLM --> |AgentCxt| AgentOutput[Agent Context]
    AgentOutput --> |ACP| AICode[AI 编码助手]
    LithoOutput --> |Search| Ask[DeepWiki 问答]
```

## 模块地图

| 模块 | 责任 | 主要路径 |
|---|---|---|
| `App` | 主界面协调，集成各组件面板 | `src/App.svelte` |
| `AskBar` | 用户提问输入组件 | `src/lib/components/AskBar.svelte` |
| `EnvIntegrate` | 扫描/初始化/集成 Git 仓库入口 | `env-catalog/agents-md/env-overview.fragment` |
| `KnowledgeGuide` | 项目知识与模块浏览 | `env-catalog/agents-md/knowledge-guide.fragment` |
| `Core::Ingest` | 仓库扫描与初始化 | `crates/mind-mesh-core/src/ingest/git.rs` |
| `Core::Doc` | Litho 文档生成逻辑 | `crates/mind-mesh-core/src/doc.rs` |
| `Core::Search` | 知识资产全文检索 | `crates/mind-mesh-core/src/search.rs` |
| `Core::Query` | LLM 问答推理与响应 | `crates/mind-mesh-core/src/query.rs` |
| `Core::Citations` | 回答引用的定位与生成 | `crates/mind-mesh-core/src/citations.rs` |
| `Agent::Context` | Agent 上下文生成与管理 | `crates/mind-mesh-agent/src/agent_context.rs` |
| `Agent::Tools` | ACP 工具调度与执行 | `crates/mind-mesh-agent/src/tools.rs` |
| `Agent::Throttle` | LLM 调用限流控制 | `crates/mind-mesh-agent/src/throttle.rs` |

## 核心流程

1.  **仓库初始化与集成**: 用户通过 CLI 或 UI 指定 Git 仓库路径。系统调用 Git CLI 扫描仓库结构，识别 `.mind-mesh/` 目录是否存在。若不存在则自动创建初始化项目结构并注册元数据。
2.  **项目扫描与打包**: 系统扫描 `crates/`, `src/` 等目录。对源码文件读取部分切片生成 `agent/repomix.md`（含文件路径、行号、内容片段）。根据 `agent/meta-inputs.md` 收集元数据，生成 `agent/meta-inputs.md` 和 `agent/context.md` 基础结构。
3.  **Litho 文档生成**: 若已集成仓库，调用 LLM 按阶段（预处理->研究->编排->输出）生成人类可读的 Markdown 文档（如 `.mind-mesh/human/` 下文档）。
4.  **DeepWiki 问答**: 用户提问时，系统在 `.mind-mesh/` 下搜索相关知识文档或 Agent Context。若匹配成功，LLM 利用上下文生成结构化回答，并自动添加代码片段引用（Citations）。

## 技术选型

*   **主要语言**: Rust (业务核心/CLI/Agent), TypeScript/Svelte (前端), Python (Litho 处理脚本 - 可选), JavaScript/TypeScript (Tauri 前端)。
*   **框架**: Tauri (桌面应用), Svelte (UI 组件), Litho (LLM 对话编排)。
*   **协议**: ACP (Agent Communication Protocol), Git, HTTP/LLM API。
*   **数据存储**: 本地文件系统 (`.mind-mesh/`)。

## 系统边界

*   **外部系统**:
    *   **Git**: 读取源码结构 (`git ls-files`, `git log` 等)。
    *   **LLM API**: 获取架构分析、文档生成、问答推理服务（Key 需用户配置）。
    *   **AI 编码助手**: 通过 ACP 协议接收查询并返回结构化 JSON 响应。
*   **信任边界**:
    *   **本地沙盒**: 仅读取指定目录，不触碰用户其他文件（通过 Tauri capability 限制）。
    *   **用户数据**: `.mind-mesh/` 下的 `human/` 目录包含本地生成的文档。

## 代码映射索引

| 概念 | 位置 | 备注 |
|---|---|---|
| `App` 入口 | `src/App.svelte` | |
| `EnvIntegratePanel` | `src/lib/components/EnvIntegratePanel.svelte` | 仓库接入入口 |
| `KnowledgeGuide` | `env-catalog/agents-md/knowledge-guide.fragment` | 模块索引 |
| `Skills` | `env-catalog/agents-md/skills.fragment` | 技能索引 |
| `Docs` | `crates/mind-mesh-core/src/doc.rs` | 文档生成入口 |
| `Ingest` | `crates/mind-mesh-core/src/ingest/` | 扫描入口 |
| `Repomix` | `crates/mind-mesh-core/src/repomix.rs` | 打包逻辑 |
| `ContextGenerator` | `crates/mind-mesh-agent/src/context_generator.rs` | Agent 上下文生成 |
| `LLMThrottle` | `crates/mind-mesh-agent/src/throttle.rs` | 限流控制 |
| `ToolExecuter` | `crates/mind-mesh-agent/src/tools.rs` | 工具执行 |
| `Search` | `crates/mind-mesh-core/src/search.rs` | 全文检索 |
| `Query` | `crates/mind-mesh-core/src/query.rs` | 问答推理 |
| `Litho::Phase1` | `preset_skills/litho-documents-skill/references/phase1-preprocessing.md` | 预处理逻辑 |
| `Litho::Phase2` | `preset_skills/litho-documents-skill/references/phase2-research.md` | 研究逻辑 |
| `Tauri::Core` | `src-tauri/src/commands.rs` | Tauri 命令定义 |

## 知识资产结构 (.mind-mesh/)

* **`.mind-mesh/agents-md/`**: * `env-overview.fragment`: 项目初始化概览 * `knowledge-guide.fragment`: 知识模块索引 * `skills.fragment`: 技能列表 * `tools.fragment`: 工具定义* **`agent/context.md`**: Agent 专用结构化知识文档（按 section 切片）* **`agent/repomix.md`**: 源码打包切片索引* **`human/`**: 人类可读的 Litho 文档（可选）* **`.mind-mesh-meta.json`**: 项目元数据配置（注入上下文）```