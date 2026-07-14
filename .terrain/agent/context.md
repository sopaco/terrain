---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手的**工程环境管理平台**：扫描 Git 仓库、生成双轨知识资产（人类 Litho C4 文档 + Agent 结构化上下文），并通过 DeepWiki 问答、SDD 四阶段工作流、Env 集成（Skills/Tools/AGENTS.md）为开发者与外部 Coding Agent 提供统一知识契约。知识存于仓库内 `.terrain/`（随分支协作）；`~/.terrain/registry.json` 仅存项目指针。消费者：桌面应用、CLI、ACP 外部 Agent（`terrain tools` JSON）。约束：宏观上下文 ≤14 KiB；源码细节在 `repomix.md` 按需检索；保鲜分低于 50 时宏观不可信。

## 架构设计

| 容器 | 职责 | 依赖 |
|------|------|------|
| **terrain-core** | 离线知识工厂：扫描、repomix 打包、搜索、保鲜、Env 应用 | Git、repomix-core、文件系统 |
| **terrain-agent** | LLM/ACP 编排：DeepWiki、Litho、SDD、agent context 生成 | terrain-core、ADK、ACP |
| **terrain-cli** | 无 UI 入口；`tools` 子命令供 ACP Agent 查询知识 | terrain-core、terrain-agent |
| **src-tauri + Svelte** | 桌面壳、Tauri IPC、项目/聊天/资产/Env UI | terrain-core、terrain-agent |
| **npm/packages** | `@terrain-ai/cli`、`@terrain-ai/rtk` 跨平台分发 | 内置二进制 shim |

**分层（知识消费）**

| 层 | 路径/API | 用途 |
|----|----------|------|
| Macro | `.terrain/agent/context.md` | 架构概览（本文档） |
| Meso | `human/`、`knowledge/`、`read-context` | 章节文档、术语 |
| Micro | `agent/repomix.md`、`grep-pack`/`read-pack-file` | 源码切片 |

**运行时数据流**：Desktop/CLI → terrain-agent（需 LLM 时）→ terrain-core → `.terrain/` / Git / registry。冲突优先级：**repomix > CodeGraph > context.md > human/**。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| terrain-core | 扫描注册、repomix 打包、全文搜索、保鲜计分、Env catalog 应用 | `crates/terrain-core/` |
| terrain-core::assets | 资产管线：context/repomix/litho/sdd、pack 读写、context 分层 | `crates/terrain-core/src/assets/` |
| terrain-core::env | Skills/Tools/AGENTS.md 检测与写入 | `crates/terrain-core/src/assets/env/` |
| terrain-agent | ChatEngine、项目初始化、Litho/SDD/上下文 ACP 会话 | `crates/terrain-agent/` |
| terrain-agent::chat | DeepWiki：native LLM 或 ACP 双后端、工具调用追踪 | `crates/terrain-agent/src/chat/` |
| terrain-cli | `scan`/`search`/`tools`/`assets`/`env` 子命令 | `crates/terrain-cli/` |
| src-tauri | AppState、Tauri command 薄封装、托盘 | `src-tauri/src/` |
| Svelte 前端 | 项目概览、DeepWiki、SDD、Env、用量、文档浏览 | `src/` |
| terrain-ts-export | ts-rs 批量导出 IPC 类型至前端 | `crates/terrain-ts-export/` |
| agent-client-protocol-tokio-patched | ACP Tokio 传输补丁（workspace patch） | `crates/agent-client-protocol-tokio-patched/` |
| preset_skills | Litho/SDD/Ask/Context 生成技能与参考 | `preset_skills/` |
| env-catalog + npm | Agent 工程环境清单与 CLI/RTK 发布 | `env-catalog/`、`npm/` |

## 核心流程

### 1. 项目初始化（知识生产）

1. `scan_project`：遍历仓库写 `index.md`，登记 `~/.terrain/registry.json`
2. `pack_agent_assets`：repomix-core 生成 `agent/repomix.md` + `meta.json`
3. `run_agent_context_generation`：LLM/ACP 写 `agent/context.md`（architecture-context 策略）
4. 可选 Litho：`run_litho_generation` 四阶段研究→编排→输出 `human/*.md`，检查点 `.terrain/.litho-agent/`
5. `compute_freshness`：对比 Git HEAD 与资产基线写 `.meta/freshness.json`

### 2. DeepWiki 知识问答

1. 前端/CLI 提交 query → `ChatEngine::ask`
2. 预载 macro：`agent/context.md` 项目概览/模块地图
3. 按需 meso：`read-context` 章节、`search`/`read-doc` 查 human/knowledge
4. micro：`grep-pack` → `read-pack-file` 读 repomix 切片；可选 CodeGraph 符号关系
5. 流式返回答案 + citations + tool_calls；LLM 失败时降级为全文搜索

### 3. Litho C4 文档生成

1. `plan_litho` 评估缺失 human 文档与 token 预算
2. ACP Agent 按 `preset_skills/litho-documents-skill` 分阶段研究（phase1–4 references）
3. 中间产物持久化 `.litho-agent/`，支持中断恢复
4. 输出六类 human 文档：概述、架构、工作流、模块、接口、数据库

### 4. SDD 标准化开发工作流

1. 四阶段顺序：需求 → 技术设计 → 代码生成（ACP）→ 代码审查
2. 阶段 1–2、4：native LLM；阶段 3：ACP Agent 改仓库
3. 会话输出 `~/.terrain/sdd/{project}/sessions/{id}/outputs/`（本地、不入库）
4. UI：`SddWorkflowPanel` + `terrain-agent/src/sdd.rs`

## 技术选型

- **语言**：Rust 2024（workspace：terrain-core/agent/cli、src-tauri）；TypeScript + Svelte 5 前端
- **桌面**：Tauri 2 + Vite 8 + Tailwind 4；IPC 类型 Rust 真源 + ts-rs → `src/lib/generated/`
- **AI 栈**：ADK（adk-model/agent/runner/session/tool）、agent-client-protocol 0.11；OpenAI/Ollama；ACP 子进程（OpenCode 等）
- **源码索引**：repomix-core 2.0（Rust 版 repomix）；pack 策略 `architecture-context`
- **关系分析**：CodeGraph CLI（bundled 至 `~/.terrain/bin/codegraph`）
- **Token 优化**：RTK（`~/.terrain/bin/rtk` 或 `@terrain-ai/rtk`）
- **分发**：npm workspace（`@terrain-ai/cli`）、平台二进制 `packages/{terrain,rtk,codegraph}/`
- **文档渲染**：marked、highlight.js、mermaid（前端 MarkdownViewer）

## 系统边界

| 边界 | 交互 | 信任 |
|------|------|------|
| **Git 仓库** | 扫描源、写 `.terrain/`、读 HEAD/脏状态 | 源码真源；知识资产随分支 |
| **LLM API** | Litho/Context/SDD 1·2·4、DeepWiki native 路径 | 需 API Key；输出需 citation 校验 |
| **ACP Agent** | Litho 编排、SDD 代码生成、Context ACP 模式、DeepWiki ACP 路径 | 子进程隔离；需 PATH 可执行体 |
| **~/.terrain/** | registry.json、sdd 会话、bin 工具链、debug | 本地私有；非知识正文 |
| **外部 Agent** | `terrain tools`（list-projects/grep-pack/read-context/freshness） | JSON stdout；不读 live FS |
| **CodeGraph** | `.codegraph/` 符号索引；`codegraph-drift` 交叉验证 | status 可能误报；以 git 漂移为准 |
| **DeepWiki MCP** | 桌面 UI 可选 GitHub 仓库文档 | 第三方；与项目 `.terrain/` 独立 |
| **env-catalog** | `terrain env apply` 注入 Skills、AGENTS.md 片段、gitignore | 依赖顺序：knowledge→repomix→codegraph→rtk |

**`.terrain/` 布局**：`index.md`、`agent/{context,repomix,meta}.md/json`、`human/`、`knowledge/`、`.meta/{sync,freshness}.json`、`.litho-agent/`。

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 知识路径解析 | `crates/terrain-core/src/paths.rs` | slug↔repo、TERRAIN_REPO_PATH |
| 项目注册表 | `crates/terrain-core/src/registry.rs` | `~/.terrain/registry.json` |
| Repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | `maybe_pack_agent_assets` |
| Agent 上下文资产 | `crates/terrain-core/src/assets/agent_context.rs` | context 状态与路径 |
| 三层消费模型 | `crates/terrain-core/src/assets/context_layers.rs` | macro/meso/micro |
| 保鲜计分 | `crates/terrain-core/src/freshness.rs` | freshness_score 规则 |
| Env 集成 catalog | `crates/terrain-core/src/assets/env/catalog.rs` | 对应 `env-catalog/catalog.json` |
| 上下文生成编排 | `crates/terrain-agent/src/context_generator.rs` | ACP/native 生成 context |
| 项目初始化流水线 | `crates/terrain-agent/src/project_init.rs` | scan+pack+context+litho |
| DeepWiki ChatEngine | `crates/terrain-agent/src/chat/mod.rs` | ask、双后端、工具追踪 |
| ACP tools CLI | `crates/terrain-cli/src/commands/tools.rs` | grep-pack/read-context 等 |
| Tauri 项目命令 | `src-tauri/src/commands/project.rs` | scan/init/freshness |
| Tauri 资产命令 | `src-tauri/src/commands/assets.rs` | pack/litho/context 生成 |
| Tauri 问答 | `src-tauri/src/commands/chat.rs` | `ask_knowledge_cmd` |
| 前端 API 桥 | `src/lib/api.ts` | invoke Tauri commands |
| IPC 类型导出 | `crates/terrain-ts-export/src/main.rs` | `bun run gen:types` |
| Agent 工作指南 | `AGENTS.md` | 知识分层、工具路径、保鲜规则 |