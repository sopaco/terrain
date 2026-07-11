---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手的**工程环境管理平台**：扫描 Git 仓库、生成双轨知识资产（人类 C4 文档 + Agent 结构化上下文），并通过 DeepWiki 问答、SDD 工作流、Env 集成三条能力线为 Coding Agent 铺平道路。核心约束：知识资产存于仓库 `.terrain/`（随分支协作）；项目登记仅在 `~/.terrain/registry.json`（指针，非正文）；宏观上下文 ≤14 KiB，实现细节在 `agent/repomix.md` 按需检索。消费者：桌面应用开发者、CLI 用户、通过 ACP 调用 `terrain tools` 的外部 Agent（Cursor、OpenCode 等）。

## 架构设计

| 容器/层 | 职责 | 主要依赖 |
|---------|------|----------|
| **桌面壳** `src-tauri/` + `src/` | Tauri 2 宿主；Svelte 5 UI；IPC 调用 Rust 后端 | `@tauri-apps/api`、Vite、Tailwind |
| **智能编排** `crates/terrain-agent/` | DeepWiki、Litho、SDD、Agent 上下文生成；Native LLM 与 ACP 双通道 | `adk-*`、`agent-client-protocol` |
| **核心引擎** `crates/terrain-core/` | 扫描/注册、repomix 打包、搜索、保鲜度、Env 计划/应用；无 LLM | `repomix-core`、walkdir |
| **CLI** `crates/terrain-cli/` + `npm/` | 无头操作；`terrain tools` JSON 出口供 ACP 消费 | terrain-core、terrain-agent |
| **类型桥** `crates/terrain-ts-export/` | Rust IPC 类型 → TypeScript（ts-rs） | ts-rs |
| **预设技能** `preset_skills/` | Litho/SDD/Ask/Context 生成工作流指令 | 随应用/CLI 捆绑 |
| **Env 目录** `env-catalog/` | Skills、工具、AGENTS.md 片段集成清单 | 本地 `~/.terrain/bin/` |

**运行时流向**：Desktop/CLI → terrain-agent（需 LLM/ACP 时）→ terrain-core → 文件系统（`.terrain/`、Git、registry）。Core 可独立离线运行；Agent 编排 LLM 轻任务与 ACP 重工具任务。

**知识三层消费模型**（DeepWiki 与 `terrain tools` 共用）：

| 层 | 来源 | 访问 |
|----|------|------|
| Macro | `agent/context.md` | 预加载 / `read-context` |
| Meso | `human/`、`knowledge/` | `search`、`read-doc` |
| Micro | `agent/repomix.md` | `grep-pack` → `read-pack-file` |

冲突优先级：**repomix > CodeGraph > context.md > human/**；`freshness_score < 50` 时降权宏观上下文。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| terrain-core | 项目扫描、注册表、路径解析、文档搜索、保鲜度 | `crates/terrain-core/src/project.rs`、`registry.rs`、`search.rs`、`freshness.rs` |
| 知识资产管线 | repomix 打包、Agent 上下文状态、Litho/SDD 资产元数据 | `crates/terrain-core/src/assets/`（`repomix.rs`、`agent_context.rs`、`litho.rs`、`sdd.rs`、`pack_read.rs`） |
| Env 集成 | 检测/计划/应用 Skills、工具、AGENTS.md 片段 | `crates/terrain-core/src/assets/env/`、`env-catalog/catalog.json` |
| terrain-agent | ChatEngine、上下文生成、项目初始化、Litho/SDD 编排 | `crates/terrain-agent/src/chat/`、`context_generator.rs`、`project_init.rs`、`litho.rs`、`sdd.rs` |
| ACP 适配 | ACP 子进程通信、工具会话缓存、兼容层 | `crates/terrain-agent/src/acp.rs`、`crates/agent-client-protocol-tokio-patched/` |
| terrain-cli | scan/search/assets/env/tools 子命令 | `crates/terrain-cli/src/cli.rs`、`commands/` |
| Tauri IPC | 前端可调用的项目/资产/聊天/SDD/Env 命令 | `src-tauri/src/commands/`（`project.rs`、`assets.rs`、`chat.rs`、`knowledge.rs`、`sdd.rs`、`env.rs`） |
| 前端 UI | 项目总览、DeepWiki、Litho 树、SDD 面板、Env 集成、用量监控 | `src/App.svelte`、`src/lib/components/`、`src/lib/stores/` |
| IPC 类型 | Rust 真源类型 + 生成 TS | `crates/terrain-core/src/schema.rs`、`crates/terrain-ts-export/`、`src/lib/types.ts` |
| npm 分发 | 跨平台 CLI shim（terrain、rtk） | `npm/packages/`、`packages/terrain/`、`packages/rtk/`、`packages/codegraph/` |
| 预设技能 | Agent 生成与人类文档工作流 | `preset_skills/`（`litho-documents-skill/`、`sdd-workflow-skill/`、`terrain-ask-skill/`、`agent-architecture-skill/`） |

## 核心流程

### 1. 项目初始化（知识工厂入口）

1. 用户选择仓库 → `scan` 遍历源码生成 `index.md` 与 `.meta/sync.json`
2. `register` 写入 `~/.terrain/registry.json`（slug ↔ repo_path）
3. `pack`（repomix-core，`architecture-context` 策略）生成 `agent/repomix.md` + `meta.json`
4. 可选：`agent-context`（LLM）生成 `agent/context.md`；`env apply` 注入 Skills/工具/AGENTS.md

### 2. Litho C4 文档生成

1. `plan-litho` 产出阶段计划；研究阶段写入 `.terrain/.litho-agent/` 检查点（可恢复）
2. ACP Agent 按 `preset_skills/litho-documents-skill/` 四阶段执行（预处理→研究→编排→输出）
3. 输出六篇人类文档至 `human/`（概述、架构、工作流、模块、接口、数据库）

### 3. DeepWiki 知识问答

1. 预加载 Macro：`agent/context.md` 概览/模块/架构
2. 按需 Meso：`read-context --section` 或 `search`/`read-doc` 查 `human/`、`knowledge/`
3. Micro：`grep-pack` 定位 → `read-pack-file` 读 repomix 切片；返回答案含引用与工具调用轨迹

### 4. SDD 标准化开发工作流

1. Phase 1–2（需求、技术设计）：Native LLM → `1.requirements.md`、`2.tech-design.md`
2. Phase 3（代码生成）：ACP Agent 改仓库 + `3.implementation.md`
3. Phase 4（代码审查）：Native LLM → `4.code-review.md`；会话输出在 `~/.terrain/sdd/{project}/sessions/`（本地、不入库）

## 技术选型

- **语言**：Rust 2024（workspace：`terrain-core`、`terrain-agent`、`terrain-cli`、`terrain-ts-export`、`src-tauri`）；TypeScript + Svelte 5 前端
- **桌面**：Tauri 2 + Vite 8 + Tailwind CSS 4
- **LLM/Agent**：`adk-core`/`adk-model`/`adk-runner`（OpenAI、Ollama）；`agent-client-protocol` + patched tokio 传输
- **源码打包**：`repomix-core` 2.x（Rust 实现 repomix）
- **IPC 类型**：ts-rs 导出；`bun run gen:types` 生成 `src/lib/generated/`
- **包管理**：Cargo workspace；Bun（前端与 npm 脚本）；`@terrain-ai/cli`、`@terrain-ai/rtk` npm 分发
- **捆绑工具**：CodeGraph（`.codegraph/` 符号索引）、RTK（shell 输出压缩）、terrain CLI
- **文档渲染**：marked、highlight.js、mermaid（前端 Markdown/Mermaid 展示）

## 系统边界

| 边界 | 类型 | 说明 |
|------|------|------|
| LLM API | 外部、需密钥 | OpenAI / Ollama 等，经 `adk-model`；桌面端配置模型与 Provider |
| ACP Agent | 外部子进程 | Litho 编排、SDD 代码生成、Context 生成（ACP 模式）；spawn 命令可配置 |
| Git 仓库 | 信任输入 | 扫描源；HEAD/dirty 状态驱动保鲜度 |
| `.terrain/` | 本地持久化、可入库 | 知识正文；`repomix.md` 常 gitignore |
| `~/.terrain/` | 本地用户态 | `registry.json`、`bin/`（terrain/rtk/codegraph）、`sdd/` 会话 |
| CodeGraph CLI | 可选外部工具 | 符号关系查询；`codegraph-drift` 交叉验证索引新鲜度 |
| RTK CLI | 可选外部工具 | 压缩 git/test/build 等 shell 输出 |
| DeepWiki MCP | 可选 UI 集成 | 桌面端 GitHub 仓库文档面板 |
| Env Skills | 注入目标仓库 | `.agents/skills/`、`.claude/skills/`；AGENTS.md 托管片段 |
| 知识保鲜 | 信任边界 | `terrain tools freshness` 重算 `.meta/freshness.json`；静态读可能过期 |

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 工作区与依赖声明 | `Cargo.toml` | workspace members、adk/repomix 版本 |
| 核心模块树 | `crates/terrain-core/src/lib.rs` | assets、ingest、project、registry、freshness 等 |
| repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | 生成 `agent/repomix.md` |
| Agent 上下文资产 | `crates/terrain-core/src/assets/agent_context.rs` | context.md 状态与元数据 |
| 上下文 LLM 生成 | `crates/terrain-agent/src/context_generator.rs` | ACP/Native 生成 context.md |
| 项目初始化编排 | `crates/terrain-agent/src/project_init.rs` | scan+pack+register 串联 |
| DeepWiki ChatEngine | `crates/terrain-agent/src/chat/mod.rs` | ask 三层检索入口 |
| Litho 生成 | `crates/terrain-agent/src/litho.rs` | 计划与执行 |
| SDD 工作流 | `crates/terrain-agent/src/sdd.rs` | 四阶段会话 |
| CLI tools 子命令 | `crates/terrain-cli/src/commands/tools.rs` | ACP 知识查询 JSON API |
| CLI assets 子命令 | `crates/terrain-cli/src/commands/assets.rs` | pack、agent-context、litho |
| Tauri 命令注册 | `src-tauri/src/lib.rs` | invoke_handler 聚合 |
| 前端 API 封装 | `src/lib/api.ts` | IPC 调用入口 |
| IPC 载荷类型 | `crates/terrain-core/src/schema.rs` | ts-export 注解真源 |
| Env 集成目录 | `env-catalog/catalog.json` | skill/tool/agents_md 依赖图 |
| 预设 Litho 技能 | `preset_skills/litho-documents-skill/SKILL.md` | 四阶段 C4 生成 |
| Agent 约定 | `AGENTS.md` | 知识分层、工具路径、保鲜规则 |