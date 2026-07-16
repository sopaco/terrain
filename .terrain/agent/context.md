---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain（Agent 地形系统）是面向 AI 编码助手时代的**工程环境管理平台**。指向 Git 仓库后，自动扫描代码、生成双轨知识资产（人类 C4 文档 + Agent 压缩架构摘要）、维护源码索引（repomix），并通过 DeepWiki 问答与 SDD 四阶段工作流，让人类开发者与外部 Coding Agent 共享同一知识契约。知识正文存于仓库 `.terrain/`（随分支流转）；`~/.terrain/registry.json` 仅存项目指针。消费者：桌面应用用户、CI/CD、`terrain tools` CLI、经 ACP 接入的外部 Agent（Cursor、OpenCode 等）。关键约束：Rust 为 IPC 类型真源；`context.md` ≤14 KiB；知识冲突优先级 repomix > CodeGraph > context.md > human/。

## 架构设计

| 容器 | 职责 | 主要路径 |
|------|------|----------|
| **Tauri 桌面壳** | 窗口、托盘、IPC 桥、sidecar 二进制 | `src-tauri/` |
| **Svelte 前端** | 项目总览、Ask、SDD、环境集成、用量 UI | `src/` |
| **terrain-core** | 扫描、知识资产、保鲜、搜索、环境目录 | `crates/terrain-core/` |
| **terrain-agent** | LLM/ACP 对话、Litho、SDD、上下文生成 | `crates/terrain-agent/` |
| **terrain-cli** | 无头扫描、知识查询、`tools` 子命令 | `crates/terrain-cli/` |
| **terrain-ts-export** | ts-rs 导出 IPC 类型至前端 | `crates/terrain-ts-export/` |
| **npm 分发层** | `@terrain-ai/cli`、`@terrain-ai/rtk` 平台 shim | `npm/` |

**分层依赖**：`src/` → Tauri `invoke` → `src-tauri/src/commands/` → `terrain-agent` / `terrain-core` → 文件系统（`.terrain/`、Git、OpenAPI）+ 外部 LLM/ACP。

**知识三层检索（Ask）**：Macro 预载 `agent/context.md` → Meso `read-context` / `search` / `read-doc` → Micro `grep-pack` → `read-pack-file`（仅读 repomix，不读活仓库）。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| **ingest** | 技术栈检测、Git 元数据、OpenAPI 导入、repomix 打包 | `crates/terrain-core/src/ingest/` |
| **assets** | Agent 上下文、Litho/SDD 计划、repomix 读取、terrain-meta 收集 | `crates/terrain-core/src/assets/` |
| **freshness** | Git HEAD 漂移检测、知识保鲜评分与快速刷新 | `crates/terrain-core/src/freshness.rs` |
| **human** | 人类 C4 文档枚举与读取 | `crates/terrain-core/src/human.rs` |
| **env 集成** | Skills/AGENTS.md 片段注入、工具链部署 | `crates/terrain-core/src/assets/env/`、`env-catalog/` |
| **chat (agent)** | DeepWiki 引擎：native LLM + ACP 双路径 | `crates/terrain-agent/src/chat/` |
| **litho** | 四阶段 C4 文档生成（断点续传） | `crates/terrain-agent/src/litho.rs`、`preset_skills/litho-documents-skill/` |
| **sdd** | 需求→设计→代码→审查工作流 | `crates/terrain-agent/src/sdd.rs`、`preset_skills/sdd-workflow-skill/` |
| **context_generator** | ACP 模式生成 `agent/context.md` | `crates/terrain-agent/src/context_generator.rs` |
| **Tauri commands** | 前端 IPC 入口（project/ask/chat/sdd/env 等） | `src-tauri/src/commands/` |
| **前端 stores** | 项目、聊天、状态、用量 Svelte 状态 | `src/lib/stores/` |
| **preset_skills** | 内置 Agent 技能（Ask、架构、Litho、SDD） | `preset_skills/` |

## 核心流程

### 1. 项目扫描与知识工厂

1. 用户选择 Git 仓库 → `ProjectScanner` 检测技术栈、采集 Git 快照、导入 OpenAPI
2. 写入 `.terrain/index.md`；`repomix-core` 生成 `agent/repomix.md`（architecture-context 策略）
3. ACP Agent 生成 `agent/context.md`；可选 Litho 产出 `human/` 六篇 C4 文档
4. `terrain-meta.json` 驱动结构化元数据；`freshness` 跟踪 pack/context/human 与 HEAD 漂移

### 2. DeepWiki 知识问答

1. 前端 `DeepWikiPanel` 经 `chat.rs` 获取/复用 `ChatEngine`（按 LLM + ACP 配置缓存）
2. Macro：`context.md` 概览/架构/模块预载；Meso：按章节或搜索 `human/`、`knowledge/`
3. Micro：`grep-pack` 定位路径 → `read-pack-file` 读 repomix 切片；附 `SourceCitation` 溯源

### 3. Litho C4 文档生成

1. 预处理 → C4 研究 → 编排 → 输出（`phase1`–`phase4` skill 引用）
2. 中间产物持久化 `.terrain/.litho-agent/` 支持断点续传
3. 最终写入 `.terrain/human/`（从 `1.概述.md` 起阅读）

### 4. SDD 标准化开发

| 阶段 | 产出 | 执行引擎 |
|------|------|----------|
| requirements | `1.requirements.md` | Native LLM |
| tech_design | `2.tech-design.md` | Native LLM |
| code_gen | `3.implementation.md` + 仓库变更 | ACP Agent |
| code_review | `4.code-review.md` | Native LLM |

会话输出存 `~/.terrain/sdd/{project}/sessions/{id}/outputs/`（本地、不入库）。

## 技术选型

- **语言/运行时**：Rust（workspace）、TypeScript、Svelte 5、Bun
- **桌面**：Tauri 2（sidecar：`terrain-cli`、`rtk`；bundled CodeGraph）
- **构建**：Vite、`svelte-check`；`bun run gen:types`（ts-rs → `src/lib/generated/`）
- **源码打包**：repomix-core（repomix-rs）
- **Agent 协议**：ACP（`agent-client-protocol-tokio-patched`）；可选 OpenCode（`adk-acp` feature）
- **配套工具**：CodeGraph（符号关系）、RTK（shell 输出压缩）、DeepWiki MCP（可选）
- **文档**：Mermaid（human/）、Markdown 全链路

## 系统边界

| 边界 | 系统/路径 | 信任与数据 |
|------|-----------|------------|
| **入站 Git** | 用户指定仓库 | 只读扫描；知识写入 `.terrain/` |
| **出站 LLM** | 用户配置的 API/本地模型 | 发送裁剪后的知识与 repomix 切片；需用户密钥 |
| **出站 ACP** | 外部 Coding Agent 进程 | Litho、context 生成、SDD codegen、Ask tool 调用 |
| **本地注册表** | `~/.terrain/registry.json` | 仅 slug↔路径；不含知识正文 |
| **本地工具链** | `~/.terrain/bin/`（terrain、codegraph、rtk） | 由应用部署；仓库内 `.terrain/env/agent-tools.json` 为本地清单 |
| **SDD 会话** | `~/.terrain/sdd/` | 工作产物，不随 Git 协作 |
| **仓库知识** | `.terrain/agent/`、`human/`、`knowledge/` | 可提交；`repomix.md` 常 gitignore |
| **外部 Agent 读码** | `terrain tools grep-pack` / `read-pack-file` | **禁止**直接读活仓库文件系统（ACP 模式契约） |
| **第三方 npm** | `@terrain-ai/cli`、`codegraph` 降级 | 无桌面安装时 `bunx`/`npx` 替代 |

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| IPC 类型真源 | `crates/terrain-core/src/schema.rs`、`crates/terrain-agent/src/chat/types.rs` | ts-rs 导出，勿手改 `generated/` |
| 项目扫描入口 | `crates/terrain-core/src/ingest/mod.rs` | `ProjectScanner`、`ScanReport` |
| repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | 策略 `architecture-context` |
| 知识路径解析 | `crates/terrain-core/src/paths.rs` | `KnowledgePaths` |
| Agent 上下文生成 | `crates/terrain-agent/src/context_generator.rs`、`agent_context.rs` | ACP 驱动 |
| Ask 三层检索 | `crates/terrain-core/src/assets/context_layers.rs` | Macro/Meso/Micro |
| 前端 API 封装 | `src/lib/api.ts` | 全部 `invoke<*>` 入口 |
| 项目总览 UI | `src/lib/components/ProjectOverviewPanel.svelte` | 保鲜、资产卡片 |
| DeepWiki UI | `src/lib/components/DeepWikiPanel.svelte` | Ask 主界面 |
| SDD UI | `src/lib/components/SddWorkflowPanel.svelte` | 四阶段 HITL |
| 环境集成 UI | `src/lib/components/EnvIntegratePanel.svelte` | Skills + AGENTS.md |
| CLI tools 子命令 | `crates/terrain-cli/src/commands/tools.rs` | `grep-pack`、`read-pack-file`、`freshness` |
| 开发者元数据 | `terrain-meta.json`（仓库根或 `.terrain/`） | 驱动模块地图与 meta-inputs |
| Agent 协作约定 | `AGENTS.md` | Terrain 注入片段（env-catalog） |
| 内置 Skills | `preset_skills/`、`env-catalog/skills/` | 注入 `.agents/skills/` |
| 类型导出二进制 | `crates/terrain-ts-export/src/main.rs` | 新增根类型须注册 `export_all_to` |