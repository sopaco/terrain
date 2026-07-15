---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手时代的**工程环境管理平台**：指向 Git 仓库后自动扫描代码、生成 C4 架构文档、维护双轨知识资产，并提供 DeepWiki 问答与 SDD 标准化工作流。消费者包括人类开发者（Tauri 桌面 / `terrain` CLI）与外部 AI Agent（`terrain tools` JSON 接口）。核心约束：知识随 Git 分支存储于 `{repo}/.terrain/`（无中心数据库）；`agent/context.md` ≤14 KiB；源码细节走 `agent/repomix.md` 按需检索；三层访问 Macro→Meso→Micro；长时任务委托 ACP，短文档可走 Native LLM；局部失败不阻断全局（新鲜度降权而非硬阻断）。

## 架构设计

| 容器/层 | 技术 | 职责 |
|--------|------|------|
| 桌面应用 | Tauri v2 + Svelte 5 | 主 UI：项目概览、DeepWiki、SDD、环境集成、用量监控 |
| CLI | `terrain-cli` (clap) | `scan`/`search`/`assets`/`env`；`tools` 子命令供 ACP Agent |
| AI 编排 | `terrain-agent` | Chat/Litho/SDD/上下文生成；LLM 与 ACP 双执行模式 |
| 知识基础设施 | `terrain-core` | 路径、扫描、搜索、资产、schema、新鲜度、repomix 打包 |
| 知识资产 | `.terrain/` | `human/` C4 文档；`agent/` 压缩上下文 + repomix；`.meta/` 同步与新鲜度 |
| 类型桥接 | `terrain-ts-export` + ts-rs | Rust→TS 单向导出 IPC 类型 |

**依赖方向**：UI/CLI → `terrain-agent` → `terrain-core` → 文件系统/Git。`agent-client-protocol-tokio-patched` 封装 ACP stdio 通信。`preset_skills/` 与 `env-catalog/` 提供 Litho/Ask/SDD 技能与环境集成模板。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|---------|
| terrain-core | 路径解析、项目注册、扫描入库、全文搜索、资产读写 | `crates/terrain-core/src/` |
| 源码扫描 | Git 元数据、技术栈检测、OpenAPI 导入、repomix 打包 | `crates/terrain-core/src/ingest/` |
| 知识资产 | Litho/上下文/repomix/SDD 资产状态与读写 | `crates/terrain-core/src/assets/` |
| 新鲜度 | 知识-源码漂移评分、信任降权 | `crates/terrain-core/src/freshness.rs` |
| terrain-agent | AI 任务编排总入口 | `crates/terrain-agent/src/lib.rs` |
| Chat 引擎 | DeepWiki 问答、工具调用、Macro 预载 | `crates/terrain-agent/src/chat/` |
| Litho 生成 | C4 文档四阶段 ACP 流水线 | `crates/terrain-agent/src/litho.rs` |
| Agent 上下文 | `context.md` 生成与就绪判定 | `crates/terrain-core/src/assets/agent_context.rs`, `crates/terrain-agent/src/agent_context.rs` |
| SDD 工作流 | 需求→设计→代码→审查四阶段 | `crates/terrain-agent/src/sdd.rs` |
| 环境集成 | Skills/CodeGraph/RTK 检测与 `AGENTS.md` 写入 | `crates/terrain-core/src/assets/env/`, `crates/terrain-agent/src/env_optimize.rs` |
| 桌面 IPC | Tauri 命令暴露给 Svelte 前端 | `src-tauri/src/commands/` |
| 前端 UI | 面板、状态、API 调用 | `src/lib/` |

## 核心流程

### 1. 项目初始化
1. 用户通过 UI 或 CLI 触发 `run_project_initialization`（`crates/terrain-agent/src/project_init.rs`）。
2. `ProjectScanner::scan_repo` 采集 Git 元数据、检测技术栈、导入 OpenAPI、打包 `agent/repomix.md`（`crates/terrain-core/src/ingest/mod.rs`）。
3. 若 `human/` 文档不完整且 ACP 可用 → `run_litho_generation` 产出 C4 文档至 `human/` 与 `.litho-agent/` 断点缓存。
4. `run_agent_context_if_needed` 生成 `agent/context.md`（Litho 刚完成时 force_refresh）。
5. 汇总 `ProjectInitResult`；非致命失败写入 notes，不 panic。

### 2. Litho 文档生成
1. 加载 `preset_skills/litho-documents-skill/`，构建 `LithoPlan`。
2. 委托 ACP Agent 执行四阶段：预处理 → C4 研究 → 编排 → 输出。
3. 研究产物持久化于 `.terrain/.litho-agent/`，支持断点续传。
4. 最终写入 `human/*.md`（六份 C4 文档）；磁盘轮询检测完成。

### 3. DeepWiki 问答（Ask）
1. `ChatEngine::run_turn` 按配置选择 Native LLM 或 Pure ACP（`crates/terrain-agent/src/chat/mod.rs`）。
2. `build_ask_prompt` 注入 Macro 层 `context.md` 概览（新鲜度不足时 withheld）。
3. Agent 按需调用工具：`read-context`（Meso）→ `grep-pack`/`read-pack-file`（Micro）。
4. 流式返回回答 + `SourceCitation` 路径引用；禁止直接读活仓库。

### 4. SDD 标准化开发
1. 四阶段：需求 → 设计 → 代码生成 → 审查（`crates/terrain-agent/src/sdd.rs`）。
2. 需求/设计/审查可走 Native LLM；CodeGen 委托 ACP Agent。
3. 会话产物存于 `~/.terrain/sdd/{project}/sessions/`；UI 通过 `src-tauri/src/commands/sdd.rs` 管理。

## 技术选型

- **核心语言**：Rust 2024 edition，Tokio 异步运行时
- **桌面壳**：Tauri v2（`src-tauri/`）
- **前端**：Svelte 5 + Vite + TypeScript（`src/`）
- **LLM 抽象**：adk-rust（OpenAI / Ollama / LM Studio）
- **ACP 通信**：agent-client-protocol 0.11.1（`crates/agent-client-protocol-tokio-patched/`）
- **源码打包**：repomix-core 2.0 → `agent/repomix.md`
- **IPC 类型**：ts-rs 单向导出至 `crates/*/bindings/`
- **存储**：文件系统 Markdown/JSON（无 SQLite/ORM）
- **分发**：`npm/packages/` 平台二进制 + `@terrain-ai/cli` shim
- **捆绑工具**：CodeGraph、RTK（`packages/`, `env-catalog/`）

## 系统边界

| 边界 | 接口 | 信任/约束 |
|------|------|----------|
| 外部 AI Agent | `terrain tools`（JSON stdout）：`read-context`/`grep-pack`/`read-pack-file`/`freshness` | 唯一推荐知识入口；禁止读活仓库 |
| 人类 CLI | `terrain scan`/`search`/`assets`/`env`/`list` | `--repo-path` 或 `TERRAIN_REPO_PATH` |
| 桌面 IPC | `src-tauri/src/commands/{knowledge,assets,chat,sdd,env,settings,project,usage}.rs` | Svelte 经 `src/lib/api.ts` 调用 |
| LLM API | HTTP（OpenAI/Ollama/LM Studio） | 配置于 `~/.terrain/settings.json` |
| ACP Agent | stdio JSON（OpenCode 等） | `acp_binary`/`TERRAIN_ACP_BINARY`；Litho/SDD CodeGen 依赖 |
| Git 仓库 | 读源码、写 `.terrain/` | 知识随分支流转 |
| 全局注册表 | `~/.terrain/registry.json` | 仅 slug→repo 指针，非知识存储 |
| repomix-core | Rust crate 库调用 | 打包策略 `architecture-context` |
| 新鲜度 | `score < 50` 时 Agent 降权信任 | 不阻断服务 |

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 知识路径解析 | `crates/terrain-core/src/paths.rs` | `KnowledgePaths` 统一 `.terrain/` 布局 |
| 项目注册表 | `crates/terrain-core/src/registry.rs` | `~/.terrain/registry.json` |
| 仓库扫描 | `crates/terrain-core/src/ingest/mod.rs` | `ProjectScanner::scan_repo` |
| repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | 产出 `agent/repomix.md` |
| 上下文分层 | `crates/terrain-core/src/assets/context_layers.rs` | Macro/Meso 字符上限契约 |
| 上下文生成 | `crates/terrain-agent/src/context_generator.rs` | LLM/ACP 双通道 |
| Ask Prompt | `crates/terrain-agent/src/chat/prompt.rs` | `build_ask_prompt` |
| ACP 执行 | `crates/terrain-agent/src/chat/acp.rs` | `run_turn_acp` |
| Litho 编排 | `crates/terrain-agent/src/litho.rs` | 四阶段流水线入口 |
| SDD 会话 | `crates/terrain-agent/src/sdd.rs` | 阶段状态机 |
| CLI 工具命令 | `crates/terrain-cli/src/commands/tools.rs` | ACP JSON 工具实现 |
| Tauri 入口 | `src-tauri/src/lib.rs` | `AppState`、命令注册 |
| 前端 API | `src/lib/api.ts` | IPC 封装 |
| 项目状态 | `src/lib/stores/project.svelte.ts` | 选中项目、新鲜度、概览 |
| 预设技能 | `preset_skills/` | Litho/Ask/SDD/Agent 架构技能 |
| 环境目录 | `env-catalog/catalog.json` | Skills/CodeGraph/RTK 集成清单 |