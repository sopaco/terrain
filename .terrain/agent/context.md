---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手的**工程环境管理平台**：扫描任意 Git 仓库、分析结构、生成 C4 人类文档与 Agent 知识资产，并支持 DeepWiki 问答与 SDD 标准化开发流程。消费者包括开发者（桌面/CLI）、DeepWiki Chat Agent、经 ACP 调用的外部编码 Agent（如 OpenCode）。核心约束：**知识原位**（`.terrain/` 随仓库演进）、**双轨文档**（`human/` 叙述性 + `agent/` 结构化）、**可恢复流水线**（Litho/SDD 断点续传）、扫描与搜索可离线、LLM/ACP 为可选增强。项目 slug 映射存于 `~/.terrain/registry.json`。

## 架构设计

| 容器/层 | 职责 | 主要路径 |
|---------|------|----------|
| **terrain-core** | 知识基础设施：扫描、打包、搜索、路径、Schema、新鲜度 | `crates/terrain-core/` |
| **terrain-agent** | AI 编排：Chat、Litho、Agent Context、SDD、ACP | `crates/terrain-agent/` |
| **terrain-cli** | 命令行入口（assets/env/knowledge/tools） | `crates/terrain-cli/` |
| **Tauri 桌面壳** | IPC 桥接、设置、预设 Skills 部署 | `src-tauri/` |
| **Svelte 前端** | DeepWiki、Litho/SDD/Env 面板、项目概览 | `src/` |
| **知识资产层** | 运行时产出，非源码 | `.terrain/agent/`、`.terrain/human/`、`.terrain/knowledge/` |

**分层依赖**：`terrain-core`（无 Agent 依赖）← `terrain-agent` ← CLI/Tauri。AI 任务分轨：轻量（问答、上下文）走 Native LLM（`ChatEngine`）；重型（Litho 全流水线、SDD 代码生成）走 ACP Agent。三层渐进取材：Macro=`agent/context.md`，Meso=`human/`+`knowledge/`，Micro=`agent/repomix.md`。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| 知识资产管理 | Repomix 打包、Litho/SDD 计划、Agent Context 元数据、Pack 读取 | `crates/terrain-core/src/assets/` |
| 源码扫描与导入 | Git 仓库扫描、OpenAPI 导入、项目索引 | `crates/terrain-core/src/ingest/` |
| 文档与搜索 | YAML frontmatter 解析、Markdown 渲染、全文搜索 | `crates/terrain-core/src/doc.rs`、`search.rs`、`human.rs` |
| 新鲜度与注册 | 项目注册表、知识路径、新鲜度账本 | `crates/terrain-core/src/registry.rs`、`freshness.rs`、`paths.rs` |
| Chat 引擎（DeepWiki） | LLM 会话、流式输出、工具调用、三层取材 | `crates/terrain-agent/src/chat/` |
| Litho 文档生成 | 四阶段 C4 流水线编排、断点续传 | `crates/terrain-agent/src/litho.rs` |
| Agent 上下文生成 | Native/ACP 双模式生成 `context.md` | `crates/terrain-agent/src/agent_context.rs` |
| 项目初始化 | 扫描→打包→Litho→上下文的完整编排 | `crates/terrain-agent/src/project_init.rs` |
| SDD 工作流 | 需求→设计→编码→评审四阶段 | `crates/terrain-agent/src/sdd.rs` |
| ACP 协议层 | OpenCode 等外部 Agent 进程通信 | `crates/terrain-agent/src/acp.rs`、`crates/agent-client-protocol-tokio-patched/` |
| 环境集成 | Skills/AGENTS.md/工具链检测与注入 | `crates/terrain-core/src/assets/env/`、`env-catalog/` |
| 桌面 UI | Tauri IPC + Svelte 5 组件与状态 | `src-tauri/src/commands/`、`src/lib/` |

## 核心流程

### 1. 项目初始化
1. 用户触发（UI 或 CLI）→ `run_project_initialization()`（`project_init.rs`）
2. `ProjectScanner::scan_repo()` 扫描技术栈，写入项目索引（纯本地）
3. `pack_agent_assets()` 用 repomix-core（`architecture-context` 策略）生成 `agent/repomix.md`
4. 若 ACP 可用 → `run_litho_generation()` 产出 `human/` 六篇 C4 文档
5. `run_agent_context_generation()` 生成 `agent/context.md` → 返回 `ProjectInitResult`

### 2. Litho 文档生成
1. `prepare_litho_generation()` 构建 `LithoPlan` 与 ACP Prompt
2. 检查 `litho_research_ready()`：研究产物在 `.terrain/.litho-agent/`，人类文档在 `human/`
3. 已就绪则跳过研究直接进入编排；否则 ACP Agent 执行预处理→C4 研究→编排→输出
4. 轮询进度（文档稳定或超时），编排最多 3 次重试补齐缺失文档

### 3. DeepWiki 问答
1. 用户提问 → Tauri `chat` 命令 → `ChatEngine::ask()`
2. Macro 层预载 `agent/context.md`；Meso 层按需 `search`/`read-doc`；Micro 层 `grep-pack`→`read-pack-file`
3. LLM 推理（OpenAI/Ollama/LM Studio）+ 工具调用追踪 → 带 `SourceCitation` 的回复

### 4. SDD 标准化开发
1. 四阶段：需求澄清→技术设计→代码生成→代码评审
2. 前三阶段评审用 Native LLM；代码生成经 ACP Agent 执行
3. 产物存 `~/.terrain/sdd/{project}/sessions/{id}/outputs/`（本地不入库）

## 技术选型

- **语言/运行时**：Rust (edition 2024) + tokio 异步
- **桌面壳**：Tauri v2（Rust IPC + 原生 WebView）
- **前端**：Svelte 5 + TypeScript + Vite + TailwindCSS
- **AI 框架**：adk-rust（Agent 会话、工具调用、流式输出）
- **源码打包**：repomix-core v2（`architecture-context` 策略）
- **知识存储**：文件系统（Markdown + YAML frontmatter + JSON 元数据）
- **CLI 解析**：clap；**TS 绑定**：ts-rs + `terrain-ts-export`
- **分发**：npm 平台包（`npm/packages/`）+ 内置二进制（`packages/terrain/`、`codegraph/`、`rtk/`）
- **图分析**：CodeGraph（`.codegraph/codegraph.db`，Bundled Tool）

## 系统边界

| 边界 | 类型 | 说明 |
|------|------|------|
| **LLM API** | 外部 HTTP | OpenAI / Ollama / LM Studio；问答、上下文生成、SDD 轻量阶段 |
| **ACP Agent（OpenCode）** | 外部进程 IPC | Litho 全流水线、SDD 代码生成、可选 Agent Context；`TERRAIN_ACP_*` 可覆盖 |
| **Git 仓库** | 只读输入 | 源码扫描、`git log` 新鲜度检测；Terrain 不替代版本控制 |
| **~/.terrain/registry.json** | 本地注册表 | slug↔repo_path 映射，可重建，不含知识正文 |
| **.terrain/** | 知识输出 | `agent/`、`human/`、`knowledge/`、`.meta/freshness.json`、`.litho-agent/` |
| **terrain CLI** | 对外接口 | `list`/`scan`/`search`/`assets`/`env`/`tools`（JSON 输出供 ACP Agent） |
| **Tauri IPC** | 桌面边界 | `src-tauri/src/commands/`：project、assets、chat、sdd、env、knowledge、settings |
| **env-catalog/** | 配置权威源 | Skills、`AGENTS.md` 片段、工具清单；同步到目标仓库 `.agents/` |
| **Bundled Tools** | 软链部署 | CodeGraph CLI、RTK CLI → `~/.terrain/bin/` |
| **信任边界** | 安全 | 核心扫描/搜索离线可用；LLM/ACP 需用户配置密钥与二进制；SDD 代码生成可写仓库（经外部 Agent） |

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 项目初始化入口 | `crates/terrain-agent/src/project_init.rs` | `run_project_initialization` |
| Repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | `pack_agent_assets` |
| Litho 编排 | `crates/terrain-agent/src/litho.rs` | `run_litho_generation`、`prepare_litho_generation` |
| Agent Context 生成 | `crates/terrain-agent/src/agent_context.rs` | Native/ACP 双模式 |
| DeepWiki Chat | `crates/terrain-agent/src/chat/` | `ChatEngine`、prompt/tracker |
| SDD 阶段执行 | `crates/terrain-agent/src/sdd.rs` | `run_sdd_phase` |
| 知识路径布局 | `crates/terrain-core/src/paths.rs` | `KnowledgePaths` |
| 项目注册与新鲜度 | `crates/terrain-core/src/registry.rs`、`freshness.rs` | slug 解析、新鲜度账本 |
| 核心类型 Schema | `crates/terrain-core/src/schema.rs` | 30+ 共享类型 |
| CLI 命令定义 | `crates/terrain-cli/src/cli.rs`、`commands/` | assets/env/knowledge/tools |
| Tauri IPC 命令 | `src-tauri/src/commands/` | 桌面↔Rust 桥接 |
| 前端 API 封装 | `src/lib/api.ts` | `invoke` 包装 |
| ACP 工具 CLI | `crates/terrain-cli/src/commands/tools.rs` | grep-pack/read-pack-file/read-context |
| 环境集成 | `crates/terrain-core/src/assets/env/` | catalog、apply、status |
| Litho Skill | `preset_skills/litho-documents-skill/` | 四阶段参考与模板 |