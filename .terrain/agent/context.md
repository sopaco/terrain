---
type: agent_context
project: terrain
title: Agent Architecture Context
source: /Users/bjsttlp485/Workspace/SAW/terrain
---

## 项目概览

Terrain 是面向 AI 编码助手的**工程环境管理平台**：自动扫描任意 Git 仓库、分析项目结构、为人类生成 C4 架构文档（Litho），同时维护 AI Agent 友好的结构化知识资产（`agent/context.md` + `agent/repomix.md`）。服务于开发者（Tauri 桌面应用）、AI 编码助手（ACP 协议 CLI）两个读者。知识资产随 Git 协作，本地 `~/.terrain/registry.json` 仅存仓库指针。

## 架构设计

```
┌─────────────────────────────────────────────────────┐
│                   桌面应用 (Tauri)                     │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────┐ │
│  │  Svelte 5 UI  │  │ Tauri 命令层  │  │  Tauri IPC │ │
│  │  (src/lib/)   │  │ (commands/)  │  │  桥接      │ │
│  └──────┬───────┘  └──────┬───────┘  └─────┬──────┘ │
└─────────┼─────────────────┼──────────────────┼───────┘
          │                 │                  │
┌─────────┼─────────────────┼──────────────────┼───────┐
│         ▼                 ▼                  ▼       │
│  ┌─────────────────────────────────────────────────┐ │
│  │              terrain-core (核心库)                │ │
│  │  知识扫描 │ 包管理 │ 资产层 │ Schema │ 项目注册  │ │
│  └──────────────────────┬──────────────────────────┘ │
│                         │                            │
│  ┌──────────────────────▼──────────────────────────┐ │
│  │             terrain-agent (AI 代理层)             │ │
│  │  Chat/Ask │ Litho 文档生成 │ Context 生成 │ SDD  │ │
│  │  ACP 执行器 │ 工具编排 │ Prompt 管理 │ 节流    │ │
│  └──────────────────────┬──────────────────────────┘ │
│                         │                            │
│  ┌──────────────────────▼──────────────────────────┐ │
│  │            LLM / ACP 后端 (adk-model)             │ │
│  │  OpenAI / Ollama / 任意 ACP 兼容端点              │ │
│  └─────────────────────────────────────────────────┘ │
│                                                       │
│  ┌──────────────┐  ┌──────────────┐                   │
│  │ terrain-cli  │  │ ts-export    │                   │
│  │ (工具命令)    │  │ (类型生成)    │                   │
│  └──────────────┘  └──────────────┘                   │
└─────────────────────────────────────────────────────┘
```

| 容器 | 职责 | 依赖 |
|------|------|------|
| Tauri 应用 (`src-tauri/`) | 桌面壳 + IPC 命令路由 | tauri 2.x, tauri-plugin-dialog/shell |
| Svelte UI (`src/`) | 项目管理、知识查看、Ask 对话、Env 集成、SDD 工作流 | Svelte 5, Tailwind, highlight.js, mermaid |
| `terrain-core` | 项目扫描、知识打包、资产层、Schema、注册表、引用管理 | adk-core, adk-acp, repomix-core, ts-rs |
| `terrain-agent` | AI 对话、Litho 文档生成、Context 生成、SDD、ACP 执行 | adk-agent, adk-model, adk-runner, adk-session |
| `terrain-cli` | 工具命令（包查询、grep、上下文读取） | terrain-core, terrain-agent |
| `terrain-ts-export` | Rust → TypeScript 类型桥接（ts-rs 导出） | ts-rs |

## 模块地图

| 模块 | 职责 | 主路径 |
|------|------|--------|
| 知识资产层 | `.terrain/` 目录管理：打包、读取、元数据 | `crates/terrain-core/src/assets/` |
| 项目初始化 | Git 仓库扫描、项目注册、索引构建 | `crates/terrain-core/src/project.rs` |
| Schema/状态 | IPC 类型定义（DocType, AgentPackMeta 等） | `crates/terrain-core/src/schema.rs` |
| 打包引擎 | 源码打包（repomix-core 集成）生成 `repomix.md` | `crates/terrain-core/src/assets/repomix.rs` |
| 引用管理 | 源码引用、人类文档引用、结构化文档引用 | `crates/terrain-core/src/citations.rs` |
| 新鲜度追踪 | 知识资产与源码的同步状态计算 | `crates/terrain-core/src/freshness.rs` |
| Chat/Ask | AI 对话引擎：ACP & Native 模式 | `crates/terrain-agent/src/chat/` |
| ACP 协议 | Agent Communication Protocol 桥接 | `crates/terrain-agent/src/acp.rs` |
| 上下文生成 | 生成 `agent/context.md` | `crates/terrain-agent/src/agent_context.rs` |
| Litho 文档 | C4 模型文档自动生成工作流 | `crates/terrain-agent/src/litho.rs` |
| SDD 工作流 | 结构决策文档工作流 | `crates/terrain-agent/src/sdd.rs` |
| 环境集成 | AI Agent 环境配置（AGENTS.md）生成 | `crates/terrain-core/src/assets/env/` |
| Tauri 命令层 | IPC 命令：project, chat, assets, sdd, settings | `src-tauri/src/commands/` |
| UI 组件 | Svelte 组件：面板、查看器、弹窗等 | `src/lib/components/` |
| 前端状态 | Svelte stores: chat, project, status | `src/lib/stores/` |
| 类型桥梁 | Rust IPC 类型 ↔ TypeScript 生成 | `crates/terrain-ts-export/` + `src/lib/generated/` |

## 核心流程

### 1. 项目初始化与扫描
1. 用户指定仓库路径 → 调用 `scan_project` / `initialize_project_cmd`
2. 仓库结构扫描（walkdir + ignore） → 识别技术栈、检测项目元数据
3. 注册到本地 `~/.terrain/registry.json`；在 `.terrain/` 下创建项目目录
4. 触发源码打包（repomix-core）→ 生成 `agent/repomix.md`
5. （可选）触发 Context 生成（LLM）→ 生成 `agent/context.md`
6. （可选）触发 Litho 文档生成（LLM）→ 生成 `human/` C4 文档

### 2. Ask / 知识问答
1. 用户输入问题（Svelte 前端或 CLI）
2. 系统搜索知识层：Macro（context.md preload）→ Meso（section 按需）→ Micro（repomix grep）
3. 构建 Prompt（含知识上下文 + 工具描述）
4. LLM 推理（ACP 或 Native 模式）：流式回答、工具调用
5. 返回结构化 `ChatReply`（答案 + 引用 + 工具记录 + Token 用量）

### 3. Litho 文档生成
1. 加载 Litho 技能 preset（相阶段模板 + LLM 指令）
2. 阶段 1 — 预处理：源码索引、调用链分析、入口点识别
3. 阶段 2 — 研究：分模块分析架构设计、依赖关系
4. 阶段 3 — 组合：按 C4 模型编排输出（项目、模块、接口、路由、事件）
5. 阶段 4 — 输出：写入 `human/` 目录并生成索引

### 4. 环境集成（Env Integrate）
1. 读取 `env-catalog/catalog.json` 中的可用技能/工具注册表
2. 根据项目需求选择集成项（skills, tools）
3. 读取对应 Fragment 模板（`env-catalog/agents-md/`）
4. 渲染合并 → 写入 `AGENTS.md`
5. Skills 注入到 `.agents/skills/` 目录

## 技术选型

- **语言**：Rust (edition 2024, rust-version 1.94) + TypeScript (Svelte 5)
- **桌面壳**：Tauri 2.x（`src-tauri/`）
- **前端**：Svelte 5 + Vite + Tailwind CSS + highlight.js + mermaid
- **AI/LLM**：adk-agent 1.0.0, adk-model (openai + ollama), adk-acp 1.0.0, agent-client-protocol 0.11.1
- **源码打包**：repomix-core 2.0
- **类型生成**：ts-rs 10（Rust → TypeScript 自动导出）
- **序列化**：serde + serde_json + serde_yaml + schemars
- **异步**：tokio (full) + futures
- **监控**：tracing + chrono + dotenvy
- **构建**：Bun (package), Cargo (workspace), Svelte-Check (typecheck)
- **包管理**：npm (src/) + Cargo (crates/), workspaces 统一

## 系统边界

| 边界 | 方向 | 协议 / 机制 | 说明 |
|------|------|-------------|------|
| LLM API | Outbound | HTTP (OpenAI / Ollama / ACP) | AI 推理；支持自定义端点 |
| Git 仓库 FS | Inbound | 本地文件系统 | 扫描与打包输入源 |
| `.terrain/` 知识目录 | 双向 | 本地文件系统（随 Git 版本化） | 知识资产持久化 |
| `~/.terrain/registry.json` | 双向 | 本地文件系统（不 commit） | 项目注册表 |
| Tauri IPC | 内部 | tauri::State + Command | Rust ↔ WebView 通信 |
| ACP CLI | Inbound | stdin/stdout JSON-RPC | AI Agent（如 OpenCode）调用 Terrain 工具 |
| 第三方 Tauri 插件 | Outbound | tauri-plugin-* | dialog, shell 系统能力 |
| 第三方 cookie crate | 内部 | patch.crates-io | 本地 patch，路径 `third-party/cookie/` |

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 工作空间根 | `/Cargo.toml` | Rust workspace, 5 members |
| 核心库入口 | `crates/terrain-core/src/lib.rs` | 扫描、打包、资产层 |
| Agent 层入口 | `crates/terrain-agent/src/lib.rs` | Chat、Litho、Context、SDD |
| IPC 类型定义 | `crates/terrain-core/src/schema.rs` | DocType, AgentPackMeta, SourceCitation 等 |
| 项目初始化 | `crates/terrain-core/src/project.rs` | scan, init, remove, freshness |
| 资产层 | `crates/terrain-core/src/assets/` | env/, pack_read, agent_context, litho, sdd |
| 源码打包 | `crates/terrain-core/src/assets/repomix.rs` | repomix-core 集成 |
| 环境集成 | `crates/terrain-core/src/assets/env/` | catalog, apply, agents_md |
| Chat 引擎 | `crates/terrain-agent/src/chat/` | acp.rs, native.rs, types.rs, prompt.rs |
| ACP 协议 | `crates/terrain-agent/src/acp.rs` | Agent Communication Protocol 桥接 |
| Context 生成 | `crates/terrain-agent/src/agent_context.rs` | 本文档的生成器 |
| CLI 入口 | `crates/terrain-cli/src/main.rs` | tools pack-meta / grep-pack / read-context 等 |
| 类型导出 | `crates/terrain-ts-export/src/main.rs` | Rust → TS 生成 |
| Tauri 应用壳 | `src-tauri/src/` | 主入口 + 命令 + 插件 |
| Tauri 命令 | `src-tauri/src/commands/` | assets, chat, env, knowledge, project, sdd, settings |
| 前端入口 | `src/App.svelte` | Svelte 根组件 |
| 前端组件 | `src/lib/components/` | 面板、查看器、对话框 |
| 前端存储 | `src/lib/stores/` | chat.svelte.ts, project.svelte.ts, status.svelte.ts |
| 前端 API | `src/lib/api.ts` | Tauri invoke 封装 |
| 生成类型 | `src/lib/generated/` | 由 bun run gen:types 生成，勿手改 |
| 前端类型 | `src/lib/types.ts` | generated + types.client 导出入口 |
| 环境编目 | `env-catalog/catalog.json` | skills/tools 注册表 |
| Preset skills | `preset_skills/` | Litho, Ask, SDD, Repomix 等预设技能 |
| 第三方 Tauri | `third-party/tauri/` | vendored 上游 |
| 第三方 cookie | `third-party/cookie/` | 本地 patch |
| AGENTS.md | `/AGENTS.md` | AI Agent 环境注入入口 |