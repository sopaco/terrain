# 桌面 UI 领域

**模块路径**：`src-tauri/src/` + `src/`
**生成日期**：2026-06-14
**分析置信度**：7/10

---

## 这个模块在做什么

桌面 UI 是 MindMesh 的"交互式驾驶舱"——由 Tauri v2 桌面壳和 Svelte 5 前端组成。Tauri 后端暴露 32 个 IPC 命令供前端调用（文档管理、AI 对话、项目初始化、SDD 工作流等），前端负责所有可视化交互。前端采用 Svelte 5（比 React 更轻量，编译为原生 DOM 操作），后端使用 Rust 提供高性能的底层能力。

---

## 核心功能点

1. **知识浏览**——ProjectOverviewPanel + HumanDocTree + KnowledgeArticle 三个组件实现从概览到详情逐层递进的知识浏览体验。
2. **DeepWiki 问答**——DeepWikiPanel 提供 AI 问答界面，带流式文本渲染、源码引用展示和工具调用可视化。
3. **SDD 工作流**——SddWorkflowPanel 提供需求→设计→编码→审查的四阶段交互式执行。
4. **环境管理**——EnvIntegratePanel 展示环境集成状态和执行入口。

## 关键组件

| 组件/类型 | 文件路径 | 核心职责 |
|---------|---------|---------|
| `App.svelte` | `src/App.svelte` | 主应用布局和状态管理 |
| `api.ts` | `src/lib/api.ts` | 32 个 Tauri invoke 命令绑定 |
| `types.ts` | `src/lib/types.ts` | TypeScript 类型定义 |
| `commands.rs` | `src-tauri/src/commands.rs` | Tauri 命令实现（819 行） |
| `AppState` | `src-tauri/src/lib.rs:8` | 应用状态管理（paths + model_config + chat） |

**分析置信度**：7/10 — 基于命令签名和组件结构分析，但大部分 Svelte 组件的内容未逐行阅读。
