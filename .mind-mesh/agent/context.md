---
type: agent_context
project: mind-mesh
title: Agent Architecture Context
source: /Users/bjsttlp485/Workspace/SAW/mind-mesh
---

## 项目概览

MindMesh 是一个面向 AI 编码助手的工程环境管理平台，致力于解决新代码库理解周期长、架构洞察难的问题。系统以 Git 仓库为输入，通过自动化流水线快速生成两套核心资产：供开发者阅读的人友好文档（基于 C4 模型和 Mermaid 图），以及供 AI 编码助手消费的结构化上下文（`agent/context.md` 与 `agent/repomix.md`）。其独特价值在于“人机双重驱动”，将传统数天的架构理解过程压缩至分钟级。

## 架构设计

MindMesh 采用“容器化”的模块化设计，核心逻辑解耦于 UI 与存储。主要技术栈包括 Rust（逻辑核心与 CLI）、Node.js/Svelte（前端界面）。数据持久化在本地 `.mind-mesh/` 目录，不依赖外部云服务。**核心依赖关系：**| 依赖项 | 类型 | 说明 |
| :--- | :--- | :--- |
| `mind-mesh-core` | Crate | 核心逻辑引擎（解析、分析、渲染） |
| `mind-mesh-agent` | Crate | AI 代理编排层（调度 Agent 生成资产） |
| `mind-mesh-cli` | Crate | 命令行入口，负责初始化和打包 |
| `rtk` | Package | 运行时框架，处理 Agent 生命周期与上下文管理 |
| `tauri` | Lib | 桌面端应用构建，提供 TUI |
| `preset_skills` | Dir | 预置 Agent 技能，驱动文档与知识生成 |
| `env-catalog` | Dir | 运行环境配置文件 |
| `third-party` | Dir | 第三方依赖（Tauri, Cookie 等） |

## 模块地图

| Module | Responsibility | Primary Paths |
| :--- | :--- | :--- |
| **MindMesh Core** | 核心逻辑引擎，处理 Git 包、源码分析、文档生成、查询 | `crates/mind-mesh-core/src/lib.rs` |
| **MindMesh Agent** | AI 代理层，负责调用预设技能调度生成 Agent 上下文 | `crates/mind-mesh-agent/src/lib.rs` |
| **MindMesh CLI** | CLI 入口，处理 `init` 与 `pack` 命令 | `crates/mind-mesh-cli/src/main.rs` |
| **Frontend (Svelte)** | 桌面应用 UI 与交互面板 | `src/App.svelte`, `src/lib/components/*.svelte` |
| **Backend (Tauri)** | 桌面端应用运行时，封装 Tauri 插件 | `src-tauri/src/lib.rs` |
| **Assets (Env)** | 环境资产处理（Agent MD, Catalog, Status） | `crates/mind-mesh-core/src/assets/env/*` |
| **Assets (Context)** | 架构上下文资产（`agent/context.md`） | `crates/mind-mesh-core/src/assets/agent_context.rs` |
| **Assets (Litho)** | 人类文档资产（`agent/context.md`） | `crates/mind-mesh-core/src/assets/litho.rs` |
| **Assets (Pack)** | 源码打包资产（`agent/repomix.md`） | `crates/mind-mesh-core/src/assets/pack_read.rs` |
| **Ingest (Git)** | Git 仓库解析与元数据提取 | `crates/mind-mesh-core/src/ingest/git.rs` |
| **Ingest (OpenAPI)** | OpenAPI 文档解析与提取 | `crates/mind-mesh-core/src/ingest/openapi.rs` |
| **Ingest (Doc)** | 人类文档（Markdown）解析与清洗 | `crates/mind-mesh-core/src/ingest/doc.rs` |
| **Assets (Search)** | 结构化索引构建，支持全文检索 | `crates/mind-mesh-core/src/search.rs` |
| **Skills (Agent)** | 预置 Agent 技能实现目录 | `preset_skills/*/SKILL.md` |
| **Citations** | 引用溯源管理 | `crates/mind-mesh-core/src/citations.rs` |

## 核心流程

1.  **项目初始化 (Project Init):** * 用户通过 CLI 或 TUI 选择目标目录。 * 触发 `git` 模块扫描，提取仓库根路径、目录结构、`Cargo.toml`/`package.json` 等元数据。 * 初始化项目上下文资产（`.mind-mesh/agent/context.md`），写入项目概览与初始架构描述。

2.  **资产打包 (Asset Packing):** * 系统遍历源码目录，将文件按路径切片，生成 `agent/repomix.md` 索引文件。 * 打包包含模块文件切片、路径映射关系及基础包结构。

3.  **知识资产生成 (Knowledge Generation):** * **文档生成：** 调用 `litho-documents-skill` 解析源码，通过 `repomix-context-skill` 获取上下文，利用 `llm` 编排生成人类可读的 C4 模型文档。 * **Agent 上下文：** 调用 `mind-mesh-ask-skill` 及 `agent-context-skill`，生成结构化的 `agent/context.md`，包含模块地图、核心流程及系统边界。 * **SDD 工作流：** 若触发，通过 `sdd-workflow-skill` 生成详细设计文档。

4.  **知识检索与问答 (Ask Mode):** * 用户通过 TUI 发起提问。 * `rtk` 层读取 `agent/context.md` 及 `agent/repomix.md` 构建上下文。 * LLM 基于上下文与搜索索引进行推理，返回带引用溯源的答案。

## 技术选型

* **编程语言:** * **Rust:** 用于核心逻辑库 (`mind-mesh-core`, `mind-mesh-cli`)

、Git 解析、OpenAPI 解析及 AI 上下文构建。强调性能与内存安全。 * **TypeScript/JavaScript:** 用于 `rtk` 运行时框架及 Node.js 脚本。 * **Svelte (TS):** 用于构建桌面应用 UI，组件化开发，高性能渲染。 * **Kotlin/Swift:** Tauri 第三方库用于移动端打包支持（非本项目直接代码）。* **框架与工具:** * **Tauri v2:** 桌面应用容器，提供 CLI 与 TUI 支持，替代 Electron 降低资源占用。 * **Svelte:** Web UI 框架。 * **Git2 / Git:** 用于 Git 仓库元数据解析与目录树遍历。 * **OpenAPI:** 内置 OpenAPI 解析器，用于从 API 文档中提取接口信息。 * **LLM (外部):** 调用外部大语言模型 API（如 GPT/others）进行架构分析与文档生成。 * **Agent Pattern:** 基于 Prompt + Skill (Skill 包含预设函数调用) 的 Agent 架构。* **数据格式:** * **Markdown:** 主要资产格式（`.md`），用于文档与上下文。 * **JSON:** 配置数据（`agent-meta.json`, `catalog.json`）。 * **Mermaid:** 架构流程图格式。

## 系统边界

* **Git 仓库 (输入)

:** 核心数据源。系统依赖 `git` 命令解析仓库结构。* **文件系统 (IO):** 所有生成的知识资产（`.mind-mesh/` 目录）及中间状态均存储于本地磁盘。* **LLM API Provider (外部依赖):** 负责架构分析、文档生成与问答推理。系统边界在此处体现为 Prompt 构建与响应解析。* **Agent Communication (ACP):** `mind-mesh-agent` 通过 ACP (Agent Communication Protocol) 调用预设技能 (`preset_skills`) 及 `mind-mesh-ask-skill` 进行处理。* **UI (TUI/Desktop):** 负责用户交互、参数配置与状态反馈，通过 Tauri 事件与后端通信。

## 代码映射索引

| Concept | Location | Notes |
| :--- | :--- | :--- |
| **App Entry** | `src/main.ts` | 应用入口 |
| **Main UI** | `src/App.svelte` | 主界面结构 |
| **CLI Binary** | `crates/mind-mesh-cli/src/main.rs` | 命令行入口 |
| **Core Lib** | `crates/mind-mesh-core/src/lib.rs` | 核心功能聚合 |
| **Agent Lib** | `crates/mind-mesh-agent/src/lib.rs` | Agent 编排逻辑 |
| **Assets Env** | `crates/mind-mesh-core/src/assets/env/mod.rs` | 环境资产接口 |
| **Assets Context** | `crates/mind-mesh-core/src/assets/agent_context.rs` | Agent 上下文生成器 |
| **Assets Litho** | `crates/mind-mesh-core/src/assets/litho.rs` | 人类文档生成器 |
| **Ingest Git** | `crates/mind-mesh-core/src/ingest/git.rs` | Git 仓库解析器 |
| **Ingest Doc** | `crates/mind-mesh-core/src/ingest/doc.rs` | 人类文档解析器 |
| **Ingest OpenAPI** | `crates/mind-mesh-core/src/ingest/openapi.rs` | OpenAPI 文档解析器 |
| **Search Index** | `crates/mind-mesh-core/src/search.rs` | 知识索引与检索 |
| **Tauri Plugin** | `src-tauri/src/commands.rs` | 桌面端命令暴露 |
| **Tauri Lib** | `src-tauri/src/lib.rs` | 桌面端逻辑聚合 |
| **Skills Root** | `preset_skills/` | 预置 Agent 技能目录 |