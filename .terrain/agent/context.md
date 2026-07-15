---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

**Terrain** 是面向 AI 编码助手时代的工程环境管理平台：指向 Git 仓库后自动扫描源码、生成 C4 架构文档、维护双轨知识资产，并通过 DeepWiki 问答与 SDD 标准化工作流，让人类开发者与外部 AI Agent 共享同一套知识契约。

**消费者**：桌面应用用户、CLI 用户、外部 ACP Agent（OpenCode/Cursor 等通过 `terrain tools` JSON 访问知识）。

**关键约束**：知识资产存于仓库内 `.terrain/`（随 Git 分支流转）；`agent/context.md` ≤14 KiB 宏观摘要，实现细节走 `agent/repomix.md` 按需检索；外部 Agent 禁止直接读活仓库；新鲜度评分 <50 时宏观上下文降权；项目登记在 `~/.terrain/registry.json`（仅存 slug→repo 指针）。

## 架构设计

### 容器与分层

| 容器 | 技术 | 职责 |
|------|------|------|
| 桌面应用 | Tauri v2 + Svelte 5 | 主交互界面，40+ IPC 命令 |
| CLI | Rust clap (`terrain-cli`) | 命令行 + ACP JSON 工具 (`terrain tools`) |
| terrain-agent | Rust + Tokio | AI 编排：Chat/Litho/SDD/ACP/项目初始化 |
| terrain-core | Rust | 知识基础设施：路径、扫描、搜索、资产、schema |
| 知识资产 `.terrain/` | Markdown/JSON | `human/` C4 文档、`agent/` 上下文+repomix、`.meta/` 元数据 |

### 依赖方向

```
UI/CLI → terrain-agent → terrain-core → Git 仓库 / .terrain/
terrain-agent → LLM API（DeepWiki/SDD 文档阶段）
terrain-agent → ACP 子进程（Litho/SDD CodeGen）
terrain-core → repomix-core（源码打包）
```

### 核心架构模式

- **分层容器**：界面与编排分离，核心能力 CLI/桌面复用
- **管道-过滤器**：Litho 四阶段、项目初始化链式执行，阶段间经文件系统传递
- **双轨知识**：`human/` 叙述性 C4 文档 vs `agent/` 压缩上下文 + repomix 索引
- **三层访问**：Macro 预载 context → Meso `read-context` 按章 → Micro `grep-pack`/`read-pack-file`
- **策略模式**：`AgentExecution` 在 Native LLM / ACP / Hybrid 间切换
- **新鲜度降权**：`FreshnessSummary` 评分过期知识，不阻断服务

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| terrain-core | 路径解析、扫描、搜索、资产、IPC schema | `crates/terrain-core/src/` |
| terrain-agent | Chat/Litho/SDD/ACP 编排与工具 | `crates/terrain-agent/src/` |
| 源码扫描 | Git 元数据、OpenAPI、repomix 打包 | `crates/terrain-core/src/ingest/` |
| 知识资产管理 | Agent 上下文、repomix、Litho/SDD 计划 | `crates/terrain-core/src/assets/` |
| Litho 文档生成 | C4 四阶段流水线（预处理→研究→编排→输出） | `crates/terrain-agent/src/litho.rs` |
| Chat 引擎 | DeepWiki 问答（Native/ACP 双后端） | `crates/terrain-agent/src/chat/` |
| SDD 工作流 | 需求→设计→代码生成→审查四阶段 | `crates/terrain-agent/src/sdd.rs` |
| 项目初始化 | 扫描→Litho→上下文一站式 onboarding | `crates/terrain-agent/src/project_init.rs` |
| 环境集成 | Skills/CodeGraph/RTK 安装与 AGENTS.md 注入 | `crates/terrain-core/src/assets/env/` |
| 新鲜度追踪 | Git 漂移检测与信任评分 | `crates/terrain-core/src/freshness.rs` |
| CLI 接口 | list/scan/search/assets/env/tools 命令组 | `crates/terrain-cli/src/` |
| 桌面 UI | Tauri 壳 + Svelte 面板 | `src-tauri/src/commands/`、`src/lib/` |

## 核心流程

### 1. 项目初始化

1. 用户通过桌面 UI 或 CLI 触发 `run_project_initialization`
2. `ProjectScanner::scan_repo` 采集 Git 元数据、技术栈、OpenAPI，并 `maybe_pack_agent_assets` 生成 repomix
3. 若 `human/` C4 文档未完成且 ACP 可用 → `run_litho_generation` 四阶段写入 `human/*.md` 与 `.litho-agent/` 研究产物
4. `run_agent_context_if_needed` 生成 ≤14 KiB 的 `agent/context.md`
5. 返回 `ProjectInitResult` 摘要；各步失败记录 notes，不 panic 全局中断

### 2. DeepWiki 问答（三层知识访问）

1. `ChatEngine::ask` 检查 pack/context 新鲜度，构建 Ask prompt
2. **Macro**：预载 `context.md` 概览（项目概览+架构+模块地图）
3. **Meso**：LLM 按需调用 `terrain tools read-context --section` 读取章节
4. **Micro**：`grep-pack` / `read-pack-file` 检索 repomix 源码切片
5. 流式返回回答 + `SourceCitation` 引用；过期知识注入信任降权块

### 3. Litho C4 文档生成

1. 预处理阶段解析仓库结构与 meta 输入
2. C4 研究阶段产出中间研究产物至 `.litho-agent/`
3. 编排阶段合成六份人类文档
4. 输出阶段写入 `human/`；`prompt_agent_with_doc_poll` 轮询磁盘，稳定后早停 ACP 会话

### 4. SDD 标准化开发

1. 四阶段状态机：需求 → 设计 → 代码生成 → 审查
2. 文档阶段（需求/设计/审查）可走 Native LLM
3. CodeGen 阶段委托 ACP Agent 执行代码变更
4. 阶段产物持久化在 `.terrain/` SDD 目录，支持会话续传

## 技术选型

- **核心语言**：Rust 2024 edition（统一 CLI/Agent/Core）
- **桌面壳**：Tauri v2（轻量跨平台，Rust 后端直连 agent）
- **前端**：Svelte 5 + Vite + Bun
- **异步运行时**：Tokio（Chat 流式、ACP 子进程、文件轮询）
- **LLM 抽象**：adk-rust（OpenAI/Ollama/LM Studio）
- **ACP 通信**：agent-client-protocol 0.11.1（`agent-client-protocol-tokio-patched` 隐藏 Windows 控制台）
- **源码打包**：repomix-core 2.0（Rust 原生 repomix-rs）
- **IPC 类型**：ts-rs 单向导出 Rust→TS（`terrain-ts-export`）
- **存储**：文件系统 Markdown/JSON（知识随 Git 流转，人类可审阅）
- **辅助工具**：CodeGraph（符号关系）、RTK（shell 输出压缩）
- **分发**：npm 平台包（`npm/packages/cli`、`rtk`）+ 预编译二进制

## 系统边界

### 外部系统

| 边界 | 交互方式 | 说明 |
|------|----------|------|
| Git 仓库 | 文件读取/写入 | 源码输入；`.terrain/` 知识输出随分支流转 |
| LLM API | HTTP（adk-rust） | DeepWiki、SDD 文档阶段推理 |
| ACP Agent（OpenCode） | stdio JSON | Litho 生成、SDD CodeGen 长时任务 |
| repomix-core | Rust crate | 打包 `agent/repomix.md` |
| `~/.terrain/registry.json` | 本地 JSON | 多项目 slug→repo 登记，不含知识正文 |
| `~/.terrain/settings.json` | 本地 JSON | LLM/ACP 配置（api_key 仅存本地） |
| `~/.terrain/bin/` | 本地二进制 | terrain/codegraph/rtk CLI 约定路径 |

### CLI 对外接口

- **项目管理**：`terrain list`、`terrain scan`、`terrain search`
- **ACP JSON 工具**（外部 Agent 唯一推荐入口）：`terrain tools list-projects`、`read-context`、`grep-pack`、`read-pack-file`、`pack-meta`、`freshness`
- **资产生成**：`terrain assets`（Litho/上下文/repomix 打包）
- **环境集成**：`terrain env`（Skills/AGENTS.md 注入）

### Tauri IPC（桌面→Rust）

| 模块 | 路径 | 职责 |
|------|------|------|
| knowledge | `src-tauri/src/commands/knowledge.rs` | 列表、扫描、搜索、文档读取 |
| assets | `src-tauri/src/commands/assets.rs` | Litho/上下文/打包生成 |
| chat | `src-tauri/src/commands/chat.rs` | DeepWiki 流式问答 |
| sdd | `src-tauri/src/commands/sdd.rs` | SDD 会话与阶段执行 |
| env | `src-tauri/src/commands/env.rs` | 环境集成计划与应用 |
| project | `src-tauri/src/commands/project.rs` | 概览、备注、初始化 |
| settings | `src-tauri/src/commands/settings.rs` | LLM/ACP 配置 |
| usage | `src-tauri/src/commands/usage.rs` | Token 用量监控 |

### 信任边界

- 外部 Agent：**必须**通过 `terrain tools` 访问知识，**禁止**直接 grep/read 活仓库
- 知识保鲜：应答前调用 `freshness`；score <50 以 repomix 为准
- 私域知识 `.terrain/knowledge/` 为人为维护，refs 失效时降权

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 知识路径解析 | `crates/terrain-core/src/paths.rs` | `KnowledgePaths`、`.terrain/` 子目录 |
| 项目扫描入口 | `crates/terrain-core/src/ingest/mod.rs` | `ProjectScanner::scan_repo` |
| Repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | `pack_agent_assets`、`maybe_pack_agent_assets` |
| Agent 上下文生成 | `crates/terrain-core/src/assets/agent_context.rs` | `write_agent_context`、章节拆分 |
| 上下文生成编排 | `crates/terrain-agent/src/context_generator.rs` | LLM/ACP prompt 构建 |
| Litho 流水线 | `crates/terrain-agent/src/litho.rs` | 四阶段 + 轮询早停 |
| Chat 引擎 | `crates/terrain-agent/src/chat/mod.rs` | `ChatEngine::ask`、三层 prompt |
| SDD 状态机 | `crates/terrain-agent/src/sdd.rs` | `run_sdd_phase`、`SddPhase` |
| 项目初始化链 | `crates/terrain-agent/src/project_init.rs` | 扫描→Litho→上下文串联 |
| ACP 配置 | `crates/terrain-agent/src/acp.rs` | `build_acp_config`、子进程通信 |
| IPC 类型契约 | `crates/terrain-core/src/schema.rs` | 跨 crate/IPC 共享类型 |
| 新鲜度评分 | `crates/terrain-core/src/freshness.rs` | `compute_freshness`、`FreshnessSummary` |
| 全文搜索 | `crates/terrain-core/src/search.rs` | `KnowledgeSearch::search` |
| 环境集成 | `crates/terrain-core/src/assets/env/apply.rs` | Skills/AGENTS.md 写入 |
| CLI 工具子命令 | `crates/terrain-cli/src/commands/tools.rs` | ACP JSON stdout 工具实现 |
| 前端 API 层 | `src/lib/api.ts` | Tauri invoke 封装 |
| Preset Skills | `preset_skills/`、`env-catalog/skills/` | Litho/Ask/SDD/架构生成技能 |