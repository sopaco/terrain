---
type: agent_context
project: mind-mesh
title: Agent Architecture Context
source: /Users/bjsttlp485/Workspace/SAW/mind-mesh
---

## 项目概览

MindMesh 是面向 AI 编码助手的**工程环境管理平台**：扫描 Git 仓库、分析结构、生成人类可读的 C4 架构文档（Litho），并维护 Agent 友好的结构化知识资产（`.mind-mesh/`）。同时服务**人类开发者**（Tauri 桌面应用 / CLI）与**外部 AI 编码助手**（通过 ACP 协议调用 `mind-mesh tools` 获取知识）。核心约束：知识资产存于仓库内 `.mind-mesh/`（可 Git 协作）；项目登记在本地 `~/.mind-mesh/registry.json`（仅路径）；Agent 上下文 ≤14 KiB；源码细节不入 context，仅存于 `agent/repomix.md` 按需检索。

## 架构设计

| 容器 | 职责 | 关键路径 |
|------|------|----------|
| **Desktop (Tauri + Svelte 5)** | 项目管理 UI、DeepWiki 问答、Litho/SDD/Env 面板 | `src-tauri/`, `src/` |
| **CLI** | 无头操作：scan/search/tools/assets/env | `crates/mind-mesh-cli/` |
| **Core** | 领域逻辑：路径、注册表、资产、搜索、新鲜度 | `crates/mind-mesh-core/` |
| **Agent** | LLM 编排：Chat、Litho、SDD、Context 生成、ACP | `crates/mind-mesh-agent/` |
| **Knowledge Store** | 分层知识：agent / human / knowledge / .meta | `{repo}/.mind-mesh/` |
| **Preset Skills** | LLM 工作流指令（Litho/SDD/Context/Ask） | `preset_skills/` |
| **Env Catalog** | Agent 工程环境集成清单（Skills/Tools/AGENTS.md） | `env-catalog/` |

**分层依赖**：UI/CLI → Agent → Core → 文件系统 / Git / LLM API。Agent 层通过 ADK（adk-agent/adk-model/adk-runner）调用 OpenAI/Ollama；ACP 模式（`adk-acp`）将工具调用委托给外部 Agent（如 OpenCode）。Core 不依赖 UI，CLI 与 Tauri 共享同一套 Core+Agent API。

**知识分层**（Agent 读取顺序）：

| 层级 | 路径 | 用途 |
|------|------|------|
| 宏观 | `.mind-mesh/agent/context.md` | 架构、模块、边界 |
| 私域 | `.mind-mesh/knowledge/` | 业务术语、内部规范 |
| 人类 | `.mind-mesh/human/` | Litho C4 文档 |
| 源码索引 | `.mind-mesh/agent/repomix.md` | 按需 grep/read（不入 LLM 上下文） |
| 关系 | `.codegraph/codegraph.db` | 符号调用链（via codegraph CLI） |

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| **mind-mesh-core** | 路径解析、项目注册、知识文档读写、Repomix 打包、搜索、新鲜度、Schema | `crates/mind-mesh-core/src/` |
| **mind-mesh-agent** | ChatEngine（DeepWiki）、Litho 生成、SDD 工作流、Agent Context 生成、项目初始化、ACP 适配 | `crates/mind-mesh-agent/src/` |
| **mind-mesh-cli** | CLI 入口：list/scan/search/read/tools/assets/env | `crates/mind-mesh-cli/src/main.rs` |
| **Tauri Backend** | IPC 命令桥接 Core+Agent；事件推送进度 | `src-tauri/src/commands.rs`, `lib.rs` |
| **Frontend UI** | Svelte 5 SPA：项目选择、Human 文档树、DeepWiki、SDD、Env 集成面板 | `src/App.svelte`, `src/lib/components/` |
| **Assets（Core）** | Agent pack/context 生成计划、Litho/SDD prompt 构建、meta 收集 | `crates/mind-mesh-core/src/assets/` |
| **Ingest** | Git 仓库扫描（ProjectScanner）、OpenAPI 摄取 | `crates/mind-mesh-core/src/ingest/` |
| **Freshness** | Git HEAD/dirty 追踪、资产新鲜度评分 | `crates/mind-mesh-core/src/freshness.rs` |
| **Env Integration** | Skills/Tools/AGENTS.md 注入与状态检测 | `crates/mind-mesh-core/src/assets/env/`, `env-catalog/` |
| **Preset Skills** | Litho 四阶段、SDD、Agent Context、Ask 工具参考 | `preset_skills/` |
| **RTK** | Shell 输出 Token 压缩包装器 | `packages/rtk/` |
| **Registry** | 本地项目登记（slug ↔ repo_path） | `crates/mind-mesh-core/src/registry.rs`, `~/.mind-mesh/registry.json` |

## 核心流程

### 1. 项目初始化（Scan → Register → Pack → Context）

1. 用户提供 `repo_path`（+ 可选 slug）
2. `ProjectScanner` 扫描仓库结构，写入 `.mind-mesh/index.md` 与 `.meta/sync.json`
3. `register_project` 登记至 `~/.mind-mesh/registry.json`
4. `pack_agent_assets` 通过 repomix-core 生成 `agent/repomix.md` + `agent/meta.json`
5. 若 context 缺失或过期，`run_agent_context_generation` 调用 LLM（Native 或 ACP 模式）生成 `agent/context.md`
6. 可选：触发 Litho 人类文档生成流水线
7. 更新 `.meta/freshness.json` 新鲜度账本

### 2. Litho 人类文档生成

1. `plan_litho_generation` 制定生成计划（research → composition）
2. Phase 1–2：LLM 研究仓库（写入 `.litho-agent/` 工作区）
3. Phase 3–4：编排 C4 文档（概述、架构、工作流、模块、接口、DB）
4. 输出至 `.mind-mesh/human/` 目录
5. Tauri 通过 `litho-progress` / `litho-done` 事件推送 UI 进度

### 3. DeepWiki 知识问答（Ask）

1. 用户在前端 AskBar 或 CLI 提交 query
2. `ChatEngine` 构建 prompt：预加载 context 宏观层 + 按需 fetch meso 层
3. LLM 可调用工具：`grep_agent_pack`、`read_agent_pack_file`、`read_agent_context`、`search_knowledge`
4. 流式返回 answer + citations + tool_calls
5. LLM 不可用时 fallback 至纯搜索

### 4. Agent 工程环境集成（Env）

1. `get_env_status` 检测 repo 中 Skills/Tools/AGENTS.md 集成状态
2. `plan_env_integration` 根据 `env-catalog/catalog.json` 生成安装计划
3. `apply_env_integration` 复制 Skills 至 `.agents/skills/`，注入 AGENTS.md 托管片段，安装 CodeGraph/RTK
4. 遵循依赖顺序：mind-mesh-knowledge → repomix → codegraph → rtk

## 技术选型

- **语言**：Rust 2024（workspace，rust-version 1.94）、TypeScript、Svelte 5
- **桌面**：Tauri 2（vendored `third-party/tauri`）、Vite 6、Tailwind CSS 4
- **LLM 框架**：ADK Rust（adk-agent/adk-model/adk-runner/adk-session/adk-tool）；模型 OpenAI + Ollama
- **ACP**：agent-client-protocol 0.11 + adk-acp（OpenCode 等外部 Agent 执行工具）
- **源码打包**：repomix-core 2.0（architecture-context 策略）
- **前端 Markdown**：marked + highlight.js + mermaid
- **包管理**：Cargo workspace、Bun（Node 工具链）
- **符号分析**：@colbymchenry/codegraph（可选集成）
- **Token 优化**：@mind-mesh/rtk（Shell 输出压缩）
- **运行时**：Tokio async、serde/serde_json、chrono、regex、walkdir

## 系统边界

| 边界 | 类型 | 说明 |
|------|------|------|
| **LLM API** | 外部 HTTP | OpenAI / Ollama；配置 via ModelSettings（.env） |
| **ACP Agent** | 外部进程 | OpenCode 等；spawn 子进程执行工具调用 |
| **Git 仓库** | 本地 FS | 扫描源、freshness 基线；只读访问 |
| **~/.mind-mesh/** | 本地 FS | registry.json、SDD sessions、debug 日志 |
| **{repo}/.mind-mesh/** | 本地 FS（可 Git） | 知识资产根；trust boundary 为项目仓库 |
| **CodeGraph** | 本地 CLI | `bunx codegraph query/callers/callees/impact` |
| **Repomix Pack** | 本地索引 | 不入 Git；由 pack 命令生成 |
| **DeepWiki MCP** | 可选外部 | 前端 DeepWikiPanel 可接 GitHub repo 文档 |
| **Tauri IPC** | 进程内 | WebView ↔ Rust backend；capability 权限控制 |
| **Env Catalog** | 内置配置 | `env-catalog/catalog.json` 定义可集成项 |

**信任模型**：Coding Agent 应优先读 `.mind-mesh/agent/context.md`；freshness_score < 50 时降权；矛盾时 repomix 源码 > codegraph > context > human。

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 知识路径解析 | `crates/mind-mesh-core/src/paths.rs` | KnowledgePaths：agent/human/litho/sdd 路径 |
| 项目注册 | `crates/mind-mesh-core/src/registry.rs` | slug ↔ repo_path 映射 |
| Agent Context 生成 | `crates/mind-mesh-agent/src/agent_context.rs`, `context_generator.rs` | Native + ACP 双模式 |
| Repomix 打包 | `crates/mind-mesh-core/src/assets/repomix.rs` | pack_agent_assets |
| Litho 流水线 | `crates/mind-mesh-agent/src/litho.rs`, `crates/mind-mesh-core/src/assets/litho.rs` | 四阶段 prompt 构建 |
| SDD 工作流 | `crates/mind-mesh-agent/src/sdd.rs`, `crates/mind-mesh-core/src/assets/sdd.rs` | 会话管理 + 阶段输出 |
| DeepWiki Chat | `crates/mind-mesh-agent/src/chat.rs`, `tools.rs` | ChatEngine + 工具 schema |
| ACP 适配 | `crates/mind-mesh-agent/src/acp.rs`, `compat_tool.rs` | 外部 Agent 工具委托 |
| 项目初始化 | `crates/mind-mesh-agent/src/project_init.rs` | scan→pack→context 编排 |
| Tauri IPC 命令 | `src-tauri/src/commands.rs` | 全部 UI 后端入口 |
| 前端 API 层 | `src/lib/api.ts` | invoke 封装 |
| CLI Tools 子命令 | `crates/mind-mesh-cli/src/main.rs` | grep-pack/read-pack-file/pack-meta |
| Env 集成 | `crates/mind-mesh-core/src/assets/env/`, `crates/mind-mesh-agent/src/env_optimize.rs` | catalog 驱动的 Skills 注入 |
| 新鲜度追踪 | `crates/mind-mesh-core/src/freshness.rs` | Git drift → 资产评分 |
| Agent 上下文 Skill | `preset_skills/agent-architecture-skill/SKILL.md` | context.md 生成契约 |
| Ask 工具参考 | `preset_skills/mind-mesh-ask-skill/SKILL.md` | CLI tools 命令文档 |