---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手的**工程环境管理平台**：扫描 Git 仓库、生成 C4 架构文档（Litho）、维护 `.terrain/` 结构化知识资产，并通过 DeepWiki 问答与 SDD 工作流支撑人机协作开发。服务两类读者：人类开发者（桌面 Tauri 应用 / CLI）与外部 AI 编码助手（ACP 协议调用 `terrain tools`）。核心约束：**知识原位**（随仓库分支流转，非中心化 DB）、**双轨文档**（`human/` 叙述性 + `agent/` 结构化）、**可恢复流水线**（Litho/SDD 中间产物持久化）、**三层知识读取**（macro 预载 context → meso 按需章节 → micro repomix grep/read）。

## 架构设计

**容器与分层**

| 层级 | 容器 | 职责 | 依赖 |
|------|------|------|------|
| 界面层 | `terrain-cli`、`src-tauri` + `src/` | CLI 命令、Tauri IPC、Svelte UI | Core + Agent |
| AI 编排层 | `terrain-agent` | Chat、Litho、SDD、ACP、上下文生成、项目初始化 | Core、LLM API、ACP Agent |
| 基础设施层 | `terrain-core` | 路径布局、注册表、扫描、搜索、知识资产、文档引擎、新鲜度 | 文件系统、Git |
| 知识存储 | `{repo}/.terrain/` | 人类文档、Agent 上下文、repomix 包、索引 | — |
| 全局注册 | `~/.terrain/registry.json` | slug ↔ 仓库路径映射 | — |

**依赖方向**：UI/CLI → Agent → Core → FS/Git；Core 不依赖 Agent 或 UI。

**主要外部依赖**：LLM API（OpenAI/Ollama/LM Studio）、ACP Agent（OpenCode 等）、repomix-core（源码打包）、adk-rust（LLM 会话）。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| terrain-core | 路径、注册、扫描、搜索、资产、schema、新鲜度 | `crates/terrain-core/src/` |
| 知识资产管理 | Agent 上下文/repomix/Litho/SDD 计划与读写 | `crates/terrain-core/src/assets/` |
| 源码扫描 | Git 仓库扫描、技术栈检测、索引生成 | `crates/terrain-core/src/ingest/` |
| terrain-agent | AI 能力编排总入口 | `crates/terrain-agent/src/` |
| Chat 引擎 | DeepWiki 问答、流式输出、工具调用追踪 | `crates/terrain-agent/src/chat/` |
| Litho 生成 | C4 文档四阶段流水线（研究→编排→输出） | `crates/terrain-agent/src/litho.rs` |
| Agent 上下文 | 生成 `agent/context.md` | `crates/terrain-agent/src/agent_context.rs` |
| 项目初始化 | 扫描→打包→Litho→上下文全流程 | `crates/terrain-agent/src/project_init.rs` |
| SDD 工作流 | 需求→设计→编码→审查四阶段 | `crates/terrain-agent/src/sdd.rs` |
| ACP 协议 | 外部 Agent 进程通信与工具桥接 | `crates/terrain-agent/src/acp.rs` |
| Tauri 后端 | IPC 命令层，桥接前端与 Rust 库 | `src-tauri/src/commands/` |
| 桌面 UI | Svelte 5 面板（Ask/Litho/SDD/Env 等） | `src/lib/components/`、`src/App.svelte` |

## 核心流程

### 1. 项目初始化

1. 用户触发（UI 或 CLI）→ `run_project_initialization()`
2. `ProjectScanner` 扫描仓库，写入 `index.md`（纯本地）
3. `pack_agent_assets()` 生成 `agent/repomix.md` + `meta.json`
4. 若 ACP 可用：`run_litho_generation()` 产出 `human/` C4 文档
5. `run_agent_context_generation()` 生成 `agent/context.md`

### 2. Litho 文档生成

1. 检查 `human/` 完整性；研究产物在 `.terrain/.litho-agent/`
2. 研究未就绪 → ACP Agent 执行预处理 + C4 研究
3. 研究就绪 → 编排阶段补齐缺失文档（最多 3 次重试）
4. 轮询文件系统检测进度；超时保留已写文档，支持断点续传

### 3. DeepWiki 问答

1. 检查 repomix/context 资产，缺失则自动补齐
2. 预载 macro 层（项目概览+架构+模块地图）构建 Prompt
3. Native LLM 或 ACP Agent 执行，可调用 `read_agent_context` / `grep-pack` / `read-pack-file`
4. 流式推送回答，提取源码引用与工具调用轨迹

### 4. SDD 四阶段工作流

1. 阶段 1–2、4：Native LLM 生成需求/设计/审查文档
2. 阶段 3：ACP Agent 执行代码生成
3. 会话产物存 `~/.terrain/sdd/{project}/sessions/{id}/outputs/`

## 技术选型

- **语言**：Rust 2024 workspace（`rust-version 1.94`）
- **桌面壳**：Tauri 2（`third-party/tauri`  vendored）
- **前端**：Svelte 5 + Vite 6 + Tailwind CSS 4 + Bun
- **CLI**：clap（`crates/terrain-cli/`）
- **LLM**：adk-rust（adk-agent/model/runner）
- **ACP**：agent-client-protocol 0.11 + adk-acp
- **源码打包**：repomix-core 2.0（`architecture-context` 策略）
- **IPC 类型**：Rust 为真源，ts-rs 导出至 `src/lib/generated/`
- **存储**：纯文件系统（Markdown + JSON），无数据库
- **可选工具**：CodeGraph（符号分析）、RTK（token 压缩，`packages/rtk/`）

## 系统边界

| 边界 | 类型 | 说明 |
|------|------|------|
| LLM API | 外部 HTTP | OpenAI / Ollama / LM Studio；需 API Key 或本地服务 |
| ACP Agent | 外部进程 | 默认 `opencode`；Litho 全流水线与 SDD 编码阶段依赖 |
| Git 仓库 | 输入源 | 扫描、repomix 打包、新鲜度基线 |
| `~/.terrain/registry.json` | 本地配置 | 项目 slug 注册表，非知识内容 |
| `terrain tools` | ACP 出口 | JSON stdout：`pack-meta`、`grep-pack`、`read-pack-file`、`read-context`、`search` |
| `terrain assets` | CLI 出口 | 注册、打包、Litho、Agent 上下文生成 |
| `terrain env` | CLI 出口 | Skills/Tools/AGENTS.md 环境集成（`env-catalog/`） |
| preset_skills | 内置资产 | Litho/SDD/Ask/Agent Context 技能定义 |
| 新鲜度 | 信任边界 | `freshness_score < 50` 时 Agent 应降低上下文权重 |
| DeepWiki MCP | 可选集成 | UI 面板支持外部仓库文档查询 |

**数据不落库**：知识在 `{repo}/.terrain/`，SDD 会话在 `~/.terrain/sdd/`。

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 知识根路径解析 | `crates/terrain-core/src/registry.rs` | `knowledge_root_for_repo()` |
| 目录布局地图 | `crates/terrain-core/src/paths.rs` | `KnowledgePaths` |
| Repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | 生成 `agent/repomix.md` |
| 上下文分层读取 | `crates/terrain-core/src/assets/context_layers.rs` | macro/meso/micro |
| 新鲜度评分 | `crates/terrain-core/src/freshness.rs` | Git HEAD + dirty 状态 |
| 全文搜索 | `crates/terrain-core/src/search.rs` | Markdown 知识库检索 |
| IPC 载荷类型 | `crates/terrain-core/src/schema.rs` | ts-export 注解 |
| Tauri 命令注册 | `src-tauri/src/lib.rs` | invoke_handler 汇总 |
| 前端 API 封装 | `src/lib/api.ts` | `invoke()` 薄封装 |
| Chat 入口 | `crates/terrain-agent/src/chat/mod.rs` | `ChatEngine::ask()` |
| Litho 编排 | `crates/terrain-agent/src/litho.rs` | `run_litho_generation()` |
| 上下文生成 | `crates/terrain-agent/src/context_generator.rs` | ACP/Native 双模式 |
| CLI 入口 | `crates/terrain-cli/src/main.rs` | 6 命令组 |
| TS 类型导出 | `crates/terrain-ts-export/src/main.rs` | `bun run gen:types` |
| 环境集成目录 | `env-catalog/catalog.json` | Skills + Tools 清单 |
| Agent 开发指南 | `AGENTS.md` | IPC 类型修改流程 |