---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手的**工程环境管理平台**：扫描 Git 仓库、生成结构化知识资产，同时服务人类开发者与 Coding Agent。输入为源码目录，输出为仓库内 `.terrain/` 下的双轨文档——叙述性 `human/`（C4 架构）与程序化 `agent/`（`context.md` + `repomix.md`）。消费者包括桌面应用、CLI、以及通过 ACP/`terrain tools` 接入的外部 AI 助手。核心约束：**知识原位**（随仓库走）、**离线优先**（扫描/搜索不依赖 LLM）、**可恢复流水线**（Litho 中间产物持久化）、分层依赖（`terrain-core` 不依赖 `terrain-agent`）。

## 架构设计

| 容器/层 | 职责 | 主要路径 |
|--------|------|---------|
| **terrain-core** | 知识基础设施：扫描、打包、搜索、路径布局、数据模型 | `crates/terrain-core/` |
| **terrain-agent** | AI 编排：Chat、Litho、上下文生成、SDD、ACP 通信 | `crates/terrain-agent/` |
| **terrain-cli** | 命令行入口（6 命令组，`terrain tools` 供 ACP） | `crates/terrain-cli/` |
| **桌面壳** | Tauri IPC + Svelte 5 富 UI | `src-tauri/`, `src/` |
| **分发层** | npm 平台二进制 shim | `npm/packages/` |
| **技能/环境目录** | 内置 preset skills、env-catalog 模板 | `preset_skills/`, `env-catalog/` |

**依赖方向**：UI/CLI → `terrain-agent` → `terrain-core` → 文件系统。重任务（Litho、SDD 编码）走 **ACP Agent**（OpenCode）；轻任务（Ask、Agent 上下文）走 **Native LLM**（`ChatEngine`）。`terrain-ts-export` 生成 TS 绑定供前端 IPC 类型安全。

**知识布局**：`{repo}/.terrain/`（项目知识根）+ `~/.terrain/registry.json`（全局项目注册表）。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|---------|
| 知识资产管理 | repomix 打包、上下文分层、Litho/SDD 计划、环境集成 | `crates/terrain-core/src/assets/` |
| 源码扫描 | Git 仓库扫描、技术栈检测、OpenAPI 导入 | `crates/terrain-core/src/ingest/` |
| 文档引擎 | YAML frontmatter + Markdown 解析/渲染 | `crates/terrain-core/src/doc.rs`, `render.rs` |
| 全文搜索 | 知识库 Markdown 全文检索 | `crates/terrain-core/src/search.rs` |
| 项目注册 | 注册表读写、项目概览聚合 | `crates/terrain-core/src/registry.rs`, `project.rs` |
| Chat 引擎 | 流式对话、工具调用、引用提取 | `crates/terrain-agent/src/chat/` |
| Litho 文档生成 | C4 文档流水线编排（预处理→研究→编排→输出） | `crates/terrain-agent/src/litho.rs` |
| Agent 上下文 | LLM/ACP 驱动的 `agent/context.md` 生成 | `crates/terrain-agent/src/agent_context.rs`, `context_generator.rs` |
| ACP 协议 | OpenCode 代理进程通信 | `crates/terrain-agent/src/acp.rs`, `chat/acp.rs` |
| 项目初始化 | 扫描→注册→打包→Litho→上下文的端到端编排 | `crates/terrain-agent/src/project_init.rs` |
| SDD 工作流 | 需求→设计→编码→审查四阶段 | `crates/terrain-agent/src/sdd.rs` |
| 桌面 UI | 项目选择、Ask、Litho/SDD 面板、环境集成 | `src/`, `src-tauri/src/commands/` |

## 核心流程

### 1. 项目初始化
1. `ProjectScanner::scan_repo()` 扫描仓库，写入索引（纯本地）
2. `pack_agent_assets()` 生成 `agent/repomix.md`
3. 注册项目到 `~/.terrain/registry.json`
4. `run_litho_generation()` 经 ACP Agent 产出 `human/` 与 `.litho-agent/` 中间物
5. `run_agent_context_generation()` 产出 `agent/context.md`

### 2. Litho 文档生成
1. `plan_litho_generation()` 检查资源与已有研究产物
2. 预处理：收集目录结构、human 摘要、developer meta
3. C4 研究阶段：ACP Agent 写入 `.terrain/.litho-agent/`（可断点续传）
4. 编排阶段：合成 `human/` 各章节（概述、架构、工作流、边界等）
5. 失败可重入：`litho_research_ready()` 跳过已完成阶段

### 3. DeepWiki 问答（Ask）
1. 用户经 UI `ask_knowledge_cmd` 或 CLI `terrain tools search` 发起查询
2. `KnowledgeSearch` 检索 `human/`、`agent/` 等知识文档
3. `ChatEngine` 组装 prompt（含 macro 层 `context.md` 预载）
4. Native LLM 流式推理，附带 `SourceCitation` 源码引用
5. 微层细节通过 `grep-pack` / `read-pack-file` 按需拉取

### 4. ACP 模式知识访问（外部 AI 助手）
1. AI 助手调用 `terrain tools list-projects` / `pack-meta`
2. 宏观：`read-context` 获取 `agent/context.md` 分段
3. 微观：`grep-pack` → `read-pack-file` 读 `agent/repomix.md` 源码片段
4. 人类文档：`read-doc` / `search` 补充叙述性上下文

## 技术选型

- **语言/运行时**：Rust（`rust-toolchain.toml`）、Tokio 异步
- **桌面壳**：Tauri v2（`src-tauri/`，crates.io；`time` pin `<0.3.48` 规避 E0119）
- **前端**：Svelte 5 + Vite + TypeScript（`src/`, `vite.config.ts`）
- **CLI**：clap（`crates/terrain-cli/`）
- **源码打包**：repomix-core（`crates/terrain-core/src/assets/repomix.rs`）
- **LLM 集成**：adk-rust ChatEngine；Provider 支持 OpenAI / Ollama / LM Studio
- **ACP**：OpenCode 子进程（`TERRAIN_ACP_BINARY` 可配置）
- **类型导出**：`terrain-ts-export` → `crates/*/bindings/`
- **包管理**：Cargo workspace + Bun/npm（`npm/` 分发平台二进制）
- **存储**：文件系统（无外部 DB）；可选 `.codegraph/codegraph.db`

## 系统边界

| 边界 | 类型 | 交互方式 | 信任/约束 |
|------|------|---------|----------|
| LLM API | 外部服务 | HTTP(S)，`ModelSettings` 配置 | API Key 存本地设置；超时/限流由 `throttle.rs` 管控 |
| OpenCode ACP | 外部进程 | stdin/stdout IPC | 仅用于 Litho/SDD 重任务；`TERRAIN_ACP_*` 可覆盖启动命令 |
| Git 仓库 | 用户数据 | 只读文件系统遍历 | 扫描/打包不修改源码；知识写入 `.terrain/` |
| 全局注册表 | 本地文件 | `~/.terrain/registry.json` | 项目 slug ↔ 仓库路径映射 |
| `terrain tools` | CLI 边界 | JSON stdout | ACP 模式专用；不直接读活仓库，仅读 repomix 包 |
| env-catalog | 内置模板 | `terrain env apply` 部署 | 写入 `.agents/`、AGENTS.md、MCP 工具配置 |
| preset_skills | 内置/可覆盖 | 环境变量指向 skill 目录 | Litho/Ask/SDD/Agent 上下文生成 prompt 来源 |
| Tauri IPC | 进程内 | 32+ invoke 命令 | 桌面 UI 唯一后端通道；`src-tauri/capabilities/` 权限控制 |
| DeepWiki MCP | 可选集成 | MCP 协议 | 外部仓库文档查询，与本地知识库互补 |

**环境变量要点**：`TERRAIN_REPO_PATH`、`TERRAIN_PROJECT_SLUG`、`TERRAIN_KNOWLEDGE_ROOT`、`TERRAIN_AGENT_CONTEXT_OUTPUT`、`TERRAIN_LITHO_WORKSPACE`（默认 `.terrain/.litho-agent/`）、`TERRAIN_HUMAN_OUTPUT_DIR`（默认 `.terrain/human/`）。

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 知识根路径解析 | `crates/terrain-core/src/registry.rs` | `knowledge_root_for_repo()` → `{repo}/.terrain` |
| 目录布局规则 | `crates/terrain-core/src/paths.rs` | `KnowledgePaths` 映射 agent/human/env 等 |
| Repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | 产出 `agent/repomix.md` |
| 上下文分层读取 | `crates/terrain-core/src/assets/context_layers.rs` | macro/meso/micro 分段供 Ask 消费 |
| Agent 上下文写入 | `crates/terrain-core/src/assets/agent_context.rs` | prompt 构建与文件落盘 |
| Developer meta 采集 | `crates/terrain-core/src/assets/project_meta.rs` | `terrain-meta.json` → `agent/meta-inputs.md` |
| 新鲜度/漂移检测 | `crates/terrain-core/src/freshness.rs` | 对比 git HEAD 与知识基线 |
| 环境集成 | `crates/terrain-core/src/assets/env/` | plan/apply/status；catalog 来自 `env-catalog/` |
| CLI 入口 | `crates/terrain-cli/src/main.rs` | list/scan/search/assets/tools/env |
| Tauri 命令注册 | `src-tauri/src/lib.rs`, `commands/mod.rs` | IPC 命令分模块：chat/knowledge/project/sdd/env |
| 前端 API 封装 | `src/lib/api.ts` | `invoke()` 包装全部 Tauri 命令 |
| Ask UI | `src/lib/components/AskBar.svelte`, `DeepWikiPanel.svelte` | 流式问答与引用展示 |
| Litho UI | `src/lib/components/ProjectOverviewPanel.svelte` | 初始化与文档生成入口 |
| SDD UI | `src/lib/components/SddWorkflowPanel.svelte` | 四阶段工作流面板 |
| TS 类型绑定 | `crates/terrain-core/bindings/`, `crates/terrain-agent/bindings/` | 由 `terrain-ts-export` 生成 |
| npm CLI shim | `npm/packages/cli/bin/terrain.js` | 平台二进制分发入口 |