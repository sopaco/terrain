---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手时代的**工程环境管理平台**：指向 Git 仓库后自动扫描结构、生成 C4 架构文档、维护双轨知识资产，并通过 DeepWiki 问答与 SDD 标准化工作流，让人类开发者与外部 Agent 共享同一套知识契约。

**消费者**：桌面用户（Tauri+Svelte）、CLI 用户、外部 ACP Agent（`terrain tools` JSON）、CI 流水线。**核心约束**：知识随代码走（`{repo}/.terrain/` 版本化）；双轨分离（`human/` 叙述性 C4 vs `agent/` ≤14KiB 压缩上下文 + repomix 按需检索）；三层访问 Macro→Meso→Micro；外部 Agent 禁止读活仓库，仅走 repomix 包。

## 架构设计

| 容器/层 | 技术 | 职责 |
|---------|------|------|
| 桌面 UI | Tauri v2 + Svelte 5 + Vite | 主交互：项目概览、DeepWiki、SDD、环境集成、用量监控 |
| CLI | `terrain-cli` (clap) | `list/scan/search` + `tools`（ACP JSON）+ `assets` + `env` |
| AI 编排 | `terrain-agent` (Tokio) | Chat/Litho/SDD/ACP 编排、项目初始化、工具 schema |
| 知识基础设施 | `terrain-core` | 路径解析、扫描、搜索、资产、schema、新鲜度 |
| 知识存储 | `.terrain/` 文件系统 | `human/`、`agent/`、`.meta/`、`.litho-agent/` |
| 全局注册表 | `~/.terrain/registry.json` | slug→repo 指针，非中心 DB |

**分层依赖**：UI/CLI → terrain-agent → terrain-core → Git 仓库 + 磁盘知识。**主要模式**：管道-过滤器（Litho 四阶段、初始化链）、策略模式（`AgentExecution` Native/ACP/Hybrid）、注册表模式（多项目无中心库）、新鲜度降权（过期知识警告不阻断）。

**关键依赖**：repomix-core（源码打包）、agent-client-protocol（ACP）、adk-rust（LLM 抽象）、ts-rs（Rust→TS IPC 类型）、Tokio。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| terrain-core | 路径、扫描、搜索、资产、schema、新鲜度 | `crates/terrain-core/src/` |
| terrain-agent | Chat/Litho/SDD/ACP 编排与项目初始化 | `crates/terrain-agent/src/` |
| 源码扫描 | Git 元数据、OpenAPI、repomix 打包 | `crates/terrain-core/src/ingest/` |
| 知识资产管理 | repomix、Agent 上下文、Litho/SDD 计划 | `crates/terrain-core/src/assets/` |
| Chat 引擎 | DeepWiki 三层问答（Native/ACP 双后端） | `crates/terrain-agent/src/chat/` |
| Litho 文档生成 | C4 四阶段流水线（预处理→研究→编排→输出） | `crates/terrain-agent/src/litho.rs` |
| SDD 工作流 | 需求→设计→代码生成→审查四阶段 | `crates/terrain-agent/src/sdd.rs` |
| 环境集成 | Skills/CodeGraph/RTK 检测与 AGENTS.md 写入 | `crates/terrain-core/src/assets/env/` |
| ACP 协议 | OpenCode 子进程配置与通信 | `crates/terrain-agent/src/acp.rs` |
| CLI 接口 | 命令行与 ACP JSON 工具集 | `crates/terrain-cli/src/` |
| 桌面壳 | Tauri IPC + Svelte 面板 | `src-tauri/src/commands/`、`src/lib/` |
| Preset Skills | 内置 Litho/Ask/SDD/架构技能 | `preset_skills/`、`env-catalog/skills/` |

## 核心流程

### 1. 项目初始化
1. 用户触发 `run_project_initialization`（UI/CLI）
2. `ProjectScanner::scan_repo` 采集 Git/OpenAPI 并打包 `agent/repomix.md`
3. 若 Litho 未完成且 ACP 可用 → 四阶段生成 `human/*.md`（中间产物 `.litho-agent/`）
4. `run_agent_context_if_needed` 生成 `agent/context.md`（≤14KiB）
5. 返回 `ProjectInitResult` 摘要

### 2. DeepWiki 问答
1. `ChatEngine::ask` 检查 pack/context 新鲜度，必要时 `prepare_agent_assets_for_ask`
2. `build_ask_prompt` 预载 Macro 层（context 概览）
3. LLM/ACP 按需调用 `read-context`（Meso）→ `grep-pack`/`read-pack-file`（Micro）
4. 流式返回回答 + `SourceCitation` 源码引用

### 3. Litho 文档生成
1. 四阶段：预处理 → C4 研究 → 编排 → 输出
2. ACP Agent 执行长时任务；`prompt_agent_with_doc_poll` 轮询磁盘产出
3. 文档集落盘且连续稳定后早停；研究产物持久化支持断点续传

### 4. SDD 标准化开发
1. 四阶段状态机：需求→设计→代码生成→审查
2. 文档阶段走 Native LLM；CodeGen 委托 ACP Agent
3. 每阶段产出可审查 Markdown，会话状态持久化

## 技术选型

- **语言**：Rust 2024 edition（CLI/Agent/Core 统一）；TypeScript/Svelte 5（前端）
- **桌面**：Tauri v2；跨平台二进制打包于 `packages/terrain/`
- **异步**：Tokio（Chat 流式、ACP 子进程、文件轮询）
- **LLM**：adk-rust 统一 OpenAI/Ollama/LM Studio
- **ACP**：agent-client-protocol 0.11.1；`agent-client-protocol-tokio-patched` 隐藏 Windows 控制台
- **源码打包**：repomix-core 2.0（`architecture-context` 策略）
- **IPC 类型**：ts-rs 单向导出至 `crates/*/bindings/`
- **存储**：Markdown/JSON 文件系统；无中心数据库
- **分发**：npm `@terrain-ai/cli` + 平台二进制；`~/.terrain/bin/terrain`
- **辅助工具**：CodeGraph（`.codegraph/`）、RTK（检索工具），经 env-catalog 集成

## 系统边界

| 边界 | 接口/协议 | 信任说明 |
|------|-----------|----------|
| Git 仓库 | 文件读写 | 源码输入；`.terrain/` 知识输出随分支流转 |
| LLM API | HTTP（OpenAI/Ollama/LM Studio） | `~/.terrain/settings.json` 存 API 密钥，仅本地 |
| ACP Agent | stdio JSON（OpenCode） | 长时 Litho/SDD CodeGen；可执行路径可配置 |
| 外部 Agent | `terrain tools` JSON stdout | **唯一推荐**知识入口；禁止读活仓库 |
| Tauri IPC | `src-tauri/src/commands/*` | 桌面 UI↔Rust；SSE 流式 Chat |
| 全局注册表 | `~/.terrain/registry.json` | 仅 slug→repo 指针 |
| env-catalog | `env-catalog/catalog.json` | Skills/工具模板与 AGENTS.md 片段 |
| repomix-core | Rust crate 依赖 | 打包库，非网络服务 |
| 新鲜度 | `terrain tools freshness` | score<50 时 Agent 降权信任，不阻断服务 |

**环境变量**：`TERRAIN_REPO_PATH`、`TERRAIN_KNOWLEDGE_ROOT`、`TERRAIN_ACP_BINARY`、`TERRAIN_LITHO_TIMEOUT_SECS`。

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 知识路径解析 | `crates/terrain-core/src/paths.rs` | `KnowledgePaths`、项目注册 |
| 仓库扫描 | `crates/terrain-core/src/ingest/mod.rs` | `ProjectScanner::scan_repo` |
| repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | `agent/repomix.md` |
| Agent 上下文 | `crates/terrain-core/src/assets/agent_context.rs` | `write_agent_context` |
| 三层上下文分层 | `crates/terrain-core/src/assets/context_layers.rs` | Macro/Meso 截取与章节提取 |
| Chat 引擎 | `crates/terrain-agent/src/chat/mod.rs` | `ChatEngine::ask` |
| Ask 提示词 | `crates/terrain-agent/src/chat/prompt.rs` | `build_ask_prompt` |
| Litho 流水线 | `crates/terrain-agent/src/litho.rs` | 四阶段 + 轮询早停 |
| SDD 状态机 | `crates/terrain-agent/src/sdd.rs` | `run_sdd_phase` |
| 项目初始化 | `crates/terrain-agent/src/project_init.rs` | 扫描→Litho→上下文链 |
| 新鲜度评分 | `crates/terrain-core/src/freshness.rs` | `FreshnessSummary`、信任块 |
| 全文搜索 | `crates/terrain-core/src/search.rs` | `KnowledgeSearch` |
| 类型契约 | `crates/terrain-core/src/schema.rs` | 跨 crate/IPC 共享类型 |
| CLI 命令定义 | `crates/terrain-cli/src/cli.rs` | `ToolsCommands`、`AssetCommands` |
| Tauri IPC | `src-tauri/src/commands/` | knowledge/assets/chat/sdd/env 等 |
| 前端 API 层 | `src/lib/api.ts` | IPC 封装与类型 |
| 环境集成 | `crates/terrain-core/src/assets/env/apply.rs` | Skills/AGENTS.md 安装 |