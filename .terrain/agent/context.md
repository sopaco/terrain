---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手时代的**工程环境管理平台**：将 Git 仓库注册为"有地图、有路标、有规范"的知识领地。核心产出存放在仓库内 `.terrain/`（随分支版本化），包括人类可读 Litho C4 文档（`human/`）、机器友好压缩上下文（`agent/context.md`、`agent/repomix.md`）与私域术语（`knowledge/`）。消费者：桌面应用（Tauri + Svelte）、`terrain` CLI、外部 Coding Agent（经 ACP `terrain tools`）。关键约束：无中心化 DB；`terrain-core` 零 UI 依赖；知识保鲜依赖 git HEAD 与新鲜度评分；Agent 上下文 ≤16 KiB。

## 架构设计

| 容器/层 | 职责 | 主要路径 |
|---------|------|----------|
| Svelte UI | DeepWiki/Litho/SDD/Env/概览面板 | `src/`, `src/lib/components/` |
| Tauri 命令层 | IPC 桥接（40+ invoke） | `src-tauri/src/commands/` |
| terrain-agent | LLM 工作流：Chat/Litho/SDD/Context | `crates/terrain-agent/` |
| terrain-core | 扫描、资产、搜索、新鲜度、IPC 类型 | `crates/terrain-core/` |
| terrain-cli | 无头 CLI + ACP tools 子命令 | `crates/terrain-cli/` |
| 知识资产 | 双轨存储与索引 | `{repo}/.terrain/` |
| preset_skills | Litho/SDD/Ask/Context 技能契约 | `preset_skills/` |

**依赖方向**：UI → Tauri → agent/core；CLI → agent/core；agent → core + LLM/ACP 外部进程。`terrain-ts-export` 将 Rust IPC 类型导出至 `src/lib/generated/`。

**核心模式**：分层架构（core→agent→cli/tauri→ui）；管道-过滤器（Litho 四阶段、三层检索）；策略模式（ChatEngine Native/ACP 双后端）；文件系统即存储。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| terrain-core | 领域逻辑：路径、扫描、资产、搜索、新鲜度 | `crates/terrain-core/src/` |
| terrain-agent | LLM 编排：问答、Litho、SDD、上下文生成 | `crates/terrain-agent/src/` |
| terrain-cli | 命令行入口与 `tools` ACP 接口 | `crates/terrain-cli/src/commands/` |
| Chat 引擎 | Native/ACP 双后端路由与流式推理 | `crates/terrain-agent/src/chat/` |
| 工作流 | init/ask/sdd/quick_refresh 编排 | `crates/terrain-agent/src/workflows/` |
| 知识资产层 | repomix 打包、context 切片、Litho 状态 | `crates/terrain-core/src/assets/` |
| 源码摄取 | Git 元数据、OpenAPI、repomix 打包 | `crates/terrain-core/src/ingest/` |
| 新鲜度 | git/codegraph 漂移检测与评分 | `crates/terrain-core/src/freshness/` |
| 环境集成 | Skills/CLI/AGENTS.md 部署计划 | `crates/terrain-core/src/assets/env/` |
| Tauri IPC | 桌面 invoke 处理器 | `src-tauri/src/commands/` |
| 前端 API | invoke 封装与状态管理 | `src/lib/api.ts`, `src/lib/stores/` |
| ACP 补丁 | Windows CREATE_NO_WINDOW 修复 | `crates/agent-client-protocol-tokio-patched/` |

## 核心流程

### 1. 项目初始化
1. 用户触发 `initialize_project`（桌面）或 `terrain init`（CLI）
2. `ProjectScanner::scan_repo` 采集 Git/OpenAPI 并 `maybe_pack_agent_assets` 生成 repomix
3. 若 `human/` 不完整 → `run_litho_generation`（ACP 四阶段 Litho）
4. 若 `agent/context.md` 缺失 → `run_agent_context_generation`（Native/ACP）
5. 返回 `ProjectInitResult`；局部失败记 notes 不中断全流程

### 2. Litho C4 文档生成
1. `prepare_litho_generation` 构建计划与 prompt（`preset_skills/litho-documents-skill/`）
2. 启动 ACP 子进程：预处理 → C4 研究 → 编排 → 输出
3. 轮询 `.terrain/.litho-agent/` 与 `human/` 进度（3s/6s 退避，默认 45min 超时）
4. 产出六份标准 `human/*.md`（含 Mermaid）

### 3. DeepWiki 知识问答（三层检索）
1. 检查新鲜度：≥50 预载 `context.md`（Macro）；<50 跳过 Macro
2. Meso：`search` human/knowledge；Micro：`grep-pack` → `read-pack-file` repomix
3. `ChatEngine` 按 `AgentExecution` 路由 Native LLM 或 ACP 子进程
4. 流式返回 `AskStreamEvent` + `SourceCitation`

### 4. SDD 标准化开发
1. 四阶段顺序：Requirements → TechDesign → CodeGen → CodeReview
2. 文档阶段（Req/TechDesign/Review）走 Native LLM；CodeGen 委托 ACP Agent
3. 产物存 `~/.terrain/sdd/{project}/sessions/{id}/outputs/`

## 技术选型

- **语言**：Rust（edition 2024 workspace）、TypeScript、Svelte 5
- **桌面**：Tauri 2（`src-tauri/`）、Vite 构建
- **异步**：Tokio 全栈；ACP 子进程经 `agent-client-protocol-tokio`（本地 patch）
- **LLM**：ADK 生态（adk-model、adk-runner）；Provider：OpenAI/Ollama/LM Studio
- **源码索引**：repomix-core（Rust 实现 repomix）→ `agent/repomix.md`
- **关系分析**：CodeGraph CLI（`packages/codegraph/`）
- **Shell 优化**：RTK CLI（`packages/rtk/`、`npm/packages/rtk/`）
- **类型共享**：ts-rs（`terrain-ts-export` → `src/lib/generated/`）
- **分发**：npm 平台包（`npm/packages/cli-*`、`terrain-*` 二进制 sidecar）
- **存储**：文件系统（Markdown/JSON），项目注册 `~/.terrain/registry.json`

## 系统边界

| 边界 | 类型 | 说明 |
|------|------|------|
| Git 仓库 | 输入 | 源码扫描、HEAD 基线、分支知识版本化 |
| LLM Provider | 外部 API | OpenAI/Ollama/LM Studio；密钥经 dotenv/`settings.json` |
| ACP Agent | 子进程 | OpenCode/Cursor CLI；Litho 编排、SDD CodeGen、重工具调用 |
| `terrain tools` | CLI/ACP 出口 | grep-pack/read-pack-file/search/read-context 供外部 Agent |
| CodeGraph | 可选 CLI | 符号关系/调用链；`~/.terrain/bin/codegraph` |
| RTK | 可选 CLI | 冗长 shell 输出压缩；`~/.terrain/bin/rtk` |
| 文件系统 | 持久化 | `.terrain/`（仓库内）、`~/.terrain/`（全局注册/设置/SDD 会话） |
| Tauri invoke | 桌面 IPC | UI ↔ Rust；类型真源在 `terrain-core/src/ipc/` |

**信任边界**：ACP 子进程隔离重工具执行；知识资产随 Git 流转非中心化；新鲜度 <50 时 Macro 上下文不可信，须 repomix/codegraph 交叉验证。

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 知识路径根抽象 | `crates/terrain-core/src/paths.rs` | `KnowledgePaths` 统一 `.terrain/` 布局 |
| 仓库扫描 | `crates/terrain-core/src/ingest/` | `ProjectScanner`, `ScanReport` |
| repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | `pack_agent_assets`, `agent_pack_ready` |
| Agent 上下文 | `crates/terrain-agent/src/agent_context.rs` | 生成 `agent/context.md` |
| 三层检索定义 | `crates/terrain-core/src/assets/context_layers.rs` | Macro/Meso/Micro 阈值与切片 |
| ChatEngine | `crates/terrain-agent/src/chat/mod.rs` | Native/ACP 路由入口 |
| Litho 生成 | `crates/terrain-agent/src/litho.rs` | ACP 四阶段轮询 |
| SDD 工作流 | `crates/terrain-agent/src/workflows/sdd.rs` | 四阶段 `run_sdd_phase` |
| 项目初始化 | `crates/terrain-agent/src/workflows/init.rs` | `run_project_initialization` |
| DeepWiki 问答 | `crates/terrain-agent/src/workflows/ask.rs` | `ask_knowledge` |
| 新鲜度评分 | `crates/terrain-core/src/freshness/` | `compute_freshness`, 阈值 50/70 |
| IPC 共享类型 | `crates/terrain-core/src/ipc/` | Litho/Ask/SDD 载荷 |
| Tauri 命令注册 | `src-tauri/src/lib.rs` | invoke 处理器列表 |
| 前端 API 封装 | `src/lib/api.ts` | 桌面 invoke 统一入口 |
| CLI 命令树 | `crates/terrain-cli/src/cli.rs` | init/ask/sdd/tools/env |
| 环境集成目录 | `env-catalog/catalog.json` | Skills/AGENTS.md 片段模板 |