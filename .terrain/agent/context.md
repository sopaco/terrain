---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手的**工程环境管理平台**：扫描 Git 仓库、生成 C4 人类文档与 Agent 结构化知识，并提供基于知识库的问答（DeepWiki）与 SDD 标准化开发工作流。消费者包括桌面用户、CLI/CI、以及通过 ACP 调用 `terrain tools` 的外部 AI 编码助手。核心约束：知识随代码存放在 `{repo}/.terrain/`；扫描与搜索可离线；LLM/ACP 为可选增强；只读分析源码（SDD CodeGen 经外部 ACP Agent 写码）。

## 架构设计

| 层级 | 容器 | 职责 |
|------|------|------|
| 界面层 | `src-tauri/` + `src/` | Tauri v2 桌面壳；Svelte 5 前端；IPC 暴露项目/聊天/资产/环境命令 |
| 界面层 | `crates/terrain-cli/` | clap CLI：`scan`/`search`/`tools`/`assets`/`env` 等 |
| AI 编排层 | `crates/terrain-agent/` | Chat、Litho、Agent 上下文、项目初始化、SDD、ACP 通信 |
| 基础设施层 | `crates/terrain-core/` | 路径布局、文档引擎、扫描/打包、搜索、Schema、注册表、新鲜度 |
| 分发层 | `npm/` + `packages/` | 跨平台 CLI shim；捆绑 `terrain`/`rtk`/`codegraph` 二进制 |
| 技能/目录 | `preset_skills/` + `env-catalog/` | Litho/Ask/SDD 等 Skill；AGENTS.md 与环境集成模板 |

**依赖方向**：Core 无 Agent 依赖 → Agent 依赖 Core → CLI/Tauri 依赖两者。知识读写经 `KnowledgePaths` 统一解析 `{repo}/.terrain/` 与 `~/.terrain/registry.json`。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| terrain-core | 知识基础设施：路径、文档、扫描、打包、搜索、类型 | `crates/terrain-core/src/` |
| 知识资产 | Agent 上下文、Litho/SDD 计划、repomix 打包与读取 | `crates/terrain-core/src/assets/` |
| 源码摄取 | Git 扫描、OpenAPI 导入 | `crates/terrain-core/src/ingest/` |
| terrain-agent | LLM/ACP 编排：Chat、文档生成、初始化、SDD | `crates/terrain-agent/src/` |
| Chat 引擎 | Native/ACP 双模式问答、流式输出、工具追踪 | `crates/terrain-agent/src/chat/` |
| Litho 生成 | C4 人类文档流水线（研究→编排→输出） | `crates/terrain-agent/src/litho.rs` |
| Agent 上下文 | 生成 `agent/context.md` | `crates/terrain-agent/src/agent_context.rs` |
| 项目初始化 | 扫描→打包→Litho→上下文 全流程 | `crates/terrain-agent/src/project_init.rs` |
| SDD 工作流 | 需求→设计→编码→审查 四阶段 | `crates/terrain-agent/src/sdd.rs` |
| ACP 协议 | OpenCode 等 ACP Agent 进程通信 | `crates/agent-client-protocol-tokio-patched/` |
| 桌面 UI | 项目总览、DeepWiki、SDD、环境集成面板 | `src/lib/components/` |
| Tauri 命令 | 前端 IPC 入口，持有 `AppState` | `src-tauri/src/commands/` |

## 核心流程

### 1. 项目初始化
1. 用户/UI/CLI 触发 `run_project_initialization()`。
2. `ProjectScanner` 扫描仓库，写入项目索引（纯本地）。
3. `pack_agent_assets()` 用 repomix-core 生成 `agent/repomix.md`。
4. 若 ACP 可用，执行 Litho 生成 `human/` C4 文档集。
5. `run_agent_context_generation()` 产出 `agent/context.md`，返回 `ProjectInitResult`。

### 2. Litho 文档生成
1. 检查 `human/` 是否已完整，完整则短路返回。
2. `litho_research_ready()` 判断研究产物；就绪则仅编排，否则全流水线。
3. 经 ACP Agent 执行 preset Litho Skill（预处理→C4 研究→编排→输出）。
4. 主进程轮询 `.terrain/.litho-agent/` 与 `human/` 写入进度（可恢复、可重试编排）。

### 3. DeepWiki 问答（Ask）
1. 前端 `DeepWikiPanel` 经 Tauri 调用 `ChatEngine`。
2. Macro 层预载 `agent/context.md`；Meso 用 `read-context`；Micro 用 `grep-pack`→`read-pack-file`。
3. Native 模式直连 LLM；ACP 模式委托外部 Agent，附源码引用 `SourceCitation`。

### 4. 环境集成
1. `env-catalog/` 定义 Skills、工具、AGENTS.md 片段。
2. `terrain env` 检测/规划/应用集成到 `~/.terrain/bin/` 与仓库 `.agents/`。
3. ACP 模式下 Ask 通过 `terrain tools` CLI 访问三层知识（与原生工具等价）。

## 技术选型

- **语言**：Rust (edition 2024) + tokio 异步运行时
- **桌面**：Tauri v2（WebView 壳）+ Svelte 5 + TypeScript + Vite + TailwindCSS
- **AI**：adk-rust（Chat/Agent 框架）；ACP（OpenCode 等）；可选 OpenAI/Ollama/LM Studio
- **源码打包**：repomix-core v2（`architecture-context` 策略）
- **CLI**：clap；跨平台分发经 `npm/packages/cli` shim
- **类型桥接**：`terrain-ts-export` + `ts-rs` 生成 `bindings/*.ts`
- **存储**：Markdown + YAML frontmatter；注册表 `~/.terrain/registry.json`；无中央 DB
- **捆绑工具**：rtk、codegraph（`packages/` 平台二进制）

## 系统边界

| 边界 | 类型 | 说明 |
|------|------|------|
| Git 仓库 | 输入（只读） | 源码扫描、`walkdir` 遍历；不替代版本控制 |
| `{repo}/.terrain/` | 本地持久化 | `agent/`、`human/`、`knowledge/`、`.litho-agent/` 等 |
| `~/.terrain/registry.json` | 本地索引 | 项目 slug→仓库路径；非知识正文 |
| LLM API | 外部 HTTP | 问答、上下文生成；可配置提供商 |
| OpenCode / ACP Agent | 外部进程 | Litho、SDD CodeGen、ACP 模式 Ask；IPC 信任边界 |
| `terrain tools` CLI | 对外 API | ACP 编码助手消费知识的三层接口 |
| preset_skills / env-catalog | 内置资源 | 随应用/CLI 分发；Litho/Ask 提示与集成模板 |
| repomix 索引 | 内部快照 | `agent/repomix.md`；Ask 禁止读活仓库文件系统 |

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 知识根路径 | `crates/terrain-core/src/registry.rs` | `knowledge_root_for_repo()` → `{repo}/.terrain` |
| 路径布局 API | `crates/terrain-core/src/paths.rs` | `KnowledgePaths`、工作区解析 |
| 核心类型 Schema | `crates/terrain-core/src/schema.rs` | Litho/SDD/新鲜度等 30+ 类型 |
| Repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | 生成 `agent/repomix.md` |
| Agent 上下文资产 | `crates/terrain-core/src/assets/agent_context.rs` | 读写 context 元数据 |
| Litho 计划/就绪检查 | `crates/terrain-core/src/assets/litho.rs` | `litho_research_ready()` |
| 全文搜索 | `crates/terrain-core/src/search.rs` | `human/`、`knowledge/` 检索 |
| CLI 命令树 | `crates/terrain-cli/src/cli.rs` | `Tools`/`Assets`/`Env` 子命令 |
| CLI 知识工具 | `crates/terrain-cli/src/commands/tools.rs` | `grep-pack`、`read-pack-file` |
| Tauri 应用状态 | `src-tauri/src/lib.rs` | `AppState`、ChatEngine 缓存 |
| Tauri IPC 命令 | `src-tauri/src/commands/` | `chat`/`project`/`assets`/`sdd`/`env` |
| 前端 API 封装 | `src/lib/api.ts` | invoke 桥接 |
| DeepWiki UI | `src/lib/components/DeepWikiPanel.svelte` | Ask 主界面 |
| Litho Skill | `preset_skills/litho-documents-skill/SKILL.md` | 四阶段文档生成规范 |
| Ask Skill (ACP) | `preset_skills/terrain-ask-skill/SKILL.md` | CLI 三层读取约定 |
| 环境目录 | `env-catalog/catalog.json` | Skills/工具/AGENTS.md 集成清单 |