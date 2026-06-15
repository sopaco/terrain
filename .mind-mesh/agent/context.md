---
type: agent_context
project: mind-mesh
title: Agent Architecture Context
source: /Users/bjsttlp485/Workspace/SAW/mind-mesh
---

## 项目概览

MindMesh 是面向 AI 编码助手的工程环境管理平台。输入为 Git 仓库源码目录，输出为结构化知识资产（C4 文档 + Agent 上下文 + Repomix）。核心能力：**多模态文档生成**（人类可读 Litho 文档 + Agent 可读结构化数据）与 **即时知识问答**。架构采用三层容器：CLI/桌面层、核心库层、Agent 代理层。主要依赖 Rust、Node.js、Svelte、Tauri 及 LLM 推理接口。

## 架构设计

系统划分为三个主要集装箱，通过 IPC 和文件 IO 通信，数据流从 Git 源码流向磁盘知识目录：

| 容器 | 角色 | 主要路径 | 输入 | 输出 | 技术栈 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `mind-mesh-cli` | CLI 入口 | `crates/mind-mesh-cli/src/main.rs` | 仓库路径 | 知识目录 | Rust |
| `mind-mesh-core` | 处理核心 | `crates/mind-mesh-core/src/lib.rs` | 解析数据 | 知识对象 | Rust |
| `mind-mesh-agent` | Agent 接口 | `crates/mind-mesh-agent/src/lib.rs` | Agent 请求 | `agent/` 目录 | Rust |
| `mind-mesh-ui` | 桌面应用 | `src-tauri/src/main.rs` | 用户指令 | UI 渲染 | Svelte/Tauri |**层依赖关系：**

- `cli` 调用 `core` 和 `ui`

- `core` 包含 `ingest` (数据摄取), `doc` (文档生成), `registry` (对象注册)

- `agent` 实现 ACP 协议，调用 `core` 搜索

## 模块地图

核心能力模块分布（仅包含核心业务模块，排除 UI 和构建工具）：

| 模块 | 职责 | 主要路径 |
| :--- | :--- | :--- |
| `env-manager` | 环境状态与目录管理 | `crates/mind-mesh-core/src/assets/env/` |
| `litho-gen` | Litho 文档多阶段生成 | `crates/mind-mesh-core/src/assets/litho.rs` |
| `context-gen` | Agent 上下文生成 | `crates/mind-mesh-agent/src/context_generator.rs` |
| `repomix-pack` | Agent 源码包生成 | `crates/mind-mesh-agent/src/agent_assets.rs` |
| `query-engine` | 知识搜索与检索 | `crates/mind-mesh-core/src/assets/query.rs` |
| `sdd-worker` | SDD 工作流执行 | `crates/mind-mesh-core/src/assets/sdd.rs` |
| `registry` | 项目元数据与注册 | `crates/mind-mesh-core/src/assets/project_meta.rs` |

## 核心流程

1. **项目初始化**：CLI 接收仓库路径 -> `ingest` 扫描代码并注册 Git 历史 -> `agent_assets` 初始化 `agent/` 目录结构 -> 完成初始化（约 1 分钟）。

2. **Litho 生成**：`context_layers` 识别层级结构 -> `litho` 调用 LLM 生成架构 Markdown -> `phaseX` 渲染具体文档 -> 输出至 `assets/litho/`。

3. **DeepWiki 问答**：`query-engine` 解析用户问题 -> `search` 在知识库索引中匹配 -> `doc` 聚合上下文 -> `render` 输出最终 Markdown。

## 技术选型

- **后端语言**：Rust (高性能并发处理，安全性)

。- **前端框架**：Svelte (桌面端 UI)。- **桌面容器**：Tauri (基于 Webview)。- **知识生成**：LLM API (架构分析，Markdown 生成)。- **文档格式**：Markdown (Litho), `.repomix` (Agent 数据)。

## 系统边界

- **信任边界**：外部 Git 仓库 -> 本地 `.mind-mesh/` 知识目录。- **网络边界**：无公网依赖，仅使用本地文件系统存储所有知识资产（LLM 调用视为外部依赖）。- **外部接口**：无直接 HTTP API（通过 ACP 协议/CLI 调用）。- **数据流**：Git 二进制/LFS -> 解析 -> 结构化文本/JSON -> 磁盘文件。

## 代码映射索引

| 概念 | 位置 | 说明 |
| :--- | :--- | :--- |
| `ProjectRoot` | `src/` | 源码目录入口 |
| `KnowledgeDir` | `.mind-mesh/knowledge/` | 知识存储根目录 |
| `AgentContextDir` | `.mind-mesh/agent/context.md` | Agent 核心上下文 |
| `RepomixDir` | `.mind-mesh/agent/repomix.md` | 源码切片包 |
| `LithoAssets` | `.mind-mesh/knowledge/` | 所有 Litho 文档 |
| `SkillCatalog` | `env-catalog/catalog.json` | 技能注册中心 |
| `CLIEntry` | `crates/mind-mesh-cli/src/main.rs` | 命令行入口点 |
| `AgentLib` | `crates/mind-mesh-agent/src/lib.rs` | Agent 能力库入口 |
| `CoreLib` | `crates/mind-mesh-core/src/lib.rs` | 核心逻辑入口 |
| `UIEntry` | `src-tauri/src/main.rs` | 桌面应用入口点 |
| `IngestGit` | `crates/mind-mesh-core/src/ingest/git.rs` | Git 历史解析器 |
| `SearchService` | `crates/mind-mesh-core/src/assets/query.rs` | 全文检索服务 |
| `ContextLayer` | `crates/mind-mesh-core/src/assets/context_layers.rs` | 架构分层识别器 |
| `LithoService` | `crates/mind-mesh-core/src/assets/litho.rs` | Litho 文档渲染器 |
| `ACPHandler` | `crates/mind-mesh-agent/src/acp.rs` | ACP 协议处理器 |
| `EnvManager` | `crates/mind-mesh-core/src/assets/env/mod.rs` | 环境状态管理器 |
| `Registery` | `crates/mind-mesh-core/src/assets/project_meta.rs` | 项目注册表 |```