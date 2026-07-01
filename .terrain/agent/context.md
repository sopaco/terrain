---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向人类开发者与 AI 编码助手的**工程环境管理平台**。输入为任意 Git 仓库，输出为仓库内 `.terrain/` 下的结构化知识资产：人类可读的 C4 Litho 文档（`human/`）、Agent 宏观架构上下文（`agent/context.md`）、源码索引包（`agent/repomix.md`）及私域术语（`knowledge/`）。消费者包括桌面应用用户、CLI 用户、以及通过 ACP 调用 `terrain tools` 的外部编码 Agent（OpenCode、Cursor 等）。核心约束：知识随代码分支走（非中心化 DB）；Rust 为真源、IPC 类型经 ts-rs 导出；扫描/搜索可离线，LLM/ACP 用于生成与问答；`freshness_score` 低时需以 repomix 交叉验证。

## 架构设计

**分层容器**

| 层 | 容器 | 职责 |
|----|------|------|
| 界面 | `src-tauri/` + `src/` | Tauri v2 桌面壳、Svelte 5 UI、Tauri IPC 命令 |
| 入口 | `crates/terrain-cli/` | CLI（list/scan/search/assets/env/tools） |
| AI 编排 | `crates/terrain-agent/` | Chat、Litho、Agent 上下文、项目初始化、SDD、ACP |
| 基础设施 | `crates/terrain-core/` | 扫描、打包、搜索、路径布局、Schema、保鲜、环境集成 |
| 协议 | `crates/agent-client-protocol-tokio-patched/` | ACP Agent 进程通信 |
| 类型桥 | `crates/terrain-ts-export/` | Rust → TypeScript IPC 类型导出 |
| 分发 | `npm/`、`packages/` | 跨平台 CLI/RTK/CodeGraph 二进制 shim |

**依赖方向**：`terrain-core` ← `terrain-agent` ← `{terrain-cli, src-tauri}`；前端经 Tauri invoke 调 Rust，不直接访问 LLM。

**三大支柱**：Knowledge（`.terrain/` 资产生产与分层消费）· Environment（Skills、`AGENTS.md`、CodeGraph/RTK）· Workflow（SDD 四阶段）。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| terrain-core | 知识目录、扫描、repomix 打包、搜索、保鲜、环境 catalog | `crates/terrain-core/src/` |
| 知识资产 | Agent 上下文层、Litho/SDD 计划、pack 读取 | `crates/terrain-core/src/assets/` |
| 源码摄取 | Git 扫描、OpenAPI 导入 | `crates/terrain-core/src/ingest/` |
| terrain-agent | LLM/ACP 编排总线 | `crates/terrain-agent/src/` |
| Chat 引擎 | DeepWiki 问答、流式输出、工具调用追踪 | `crates/terrain-agent/src/chat/` |
| Litho 生成 | C4 文档四阶段流水线（可断点续传） | `crates/terrain-agent/src/litho.rs` |
| 项目初始化 | 扫描→打包→Litho→context 全流程 | `crates/terrain-agent/src/project_init.rs` |
| SDD 工作流 | 需求→设计→编码(ACP)→审查 | `crates/terrain-agent/src/sdd.rs` |
| ACP 集成 | OpenCode 等外部 Agent 代理 | `crates/terrain-agent/src/acp.rs` |
| Tauri 命令层 | 桌面 IPC 暴露 core/agent 能力 | `src-tauri/src/commands/` |
| Svelte 前端 | 项目总览、DeepWiki、SDD、环境集成 UI | `src/lib/components/` |
| CLI / ACP tools | 人机命令与 JSON 工具接口 | `crates/terrain-cli/src/commands/` |

## 核心流程

### 1. 项目初始化

1. 用户触发 UI 或 `terrain assets` 相关命令
2. `ProjectScanner` 扫描仓库技术栈与结构（纯本地）
3. `pack_agent_assets` 生成 `agent/repomix.md`
4. 可选：`run_litho_generation` 经 ACP 产出 `human/` 与 `.litho-agent/` 检查点
5. `run_agent_context_generation` 生成 `agent/context.md`
6. 返回 `ProjectInitResult`（文件数、token、完整性）

### 2. Litho 文档生成

1. 检查 `human/` 完整性；研究产物齐全则跳过研究
2. `prepare_litho_generation` 构建 `LithoPlan` 与 ACP Prompt
3. ACP Agent 执行：预处理 → C4 研究 → 编排 → 输出
4. 轮询文件系统（约 3s）直至稳定或超时（默认 45min）
5. 编排阶段最多 3 次重试补齐缺失文档

### 3. DeepWiki 问答（Ask）

1. 检查 repomix/context 就绪，缺失则自动补全
2. 预加载 macro 层（`context.md` 概览/架构/模块地图）
3. Native LLM 或 ACP Agent 执行，可按需调用 meso（`read-context`）与 micro（`grep-pack`/`read-pack-file`）
4. 流式推送回答并提取 `SourceCitation`

### 4. SDD 标准化开发

1. 阶段 1–2：Native LLM 产出需求与设计 Markdown
2. 阶段 3：ACP Agent 编码并写 `implementation.md`
3. 阶段 4：Native LLM 代码审查
4. 会话输出存 `~/.terrain/sdd/{project}/sessions/{id}/outputs/`（本地、不入库）

## 技术选型

- **语言**：Rust（core/agent/cli/tauri）、TypeScript/Svelte 5（UI）
- **桌面**：Tauri v2 + Vite；IPC 载荷 Rust 为真源，ts-rs 生成 `src/lib/generated/`
- **异步**：Tokio；LLM 经 HTTP（OpenAI/Ollama/LM Studio）
- **CLI**：clap；`terrain tools` 输出 JSON 供 ACP
- **打包**：repomix-core（`architecture-context` 策略）
- **前端工具链**：Bun、Vite、Svelte 5 runes
- **存储**：文件系统——`.terrain/`（仓库内）、`~/.terrain/registry.json`（项目指针）、`~/.terrain/sdd/`（SDD 会话）
- **配套工具**：CodeGraph（符号关系）、RTK（shell 输出压缩）、preset/env-catalog Skills

## 系统边界

| 边界 | 类型 | 说明 |
|------|------|------|
| LLM API | 外部 HTTP | OpenAI / Ollama / LM Studio；需 API Key 或本地服务 |
| ACP Agent | 外部进程 | 默认 `opencode`；Litho 与 SDD 编码阶段；`TERRAIN_ACP_*` 可配置 |
| Git 仓库 | 输入源 | 扫描、repomix 打包、保鲜基线 `baseline_git_head` |
| `~/.terrain/registry.json` | 本地注册表 | 仅路径索引，不含知识正文 |
| `terrain tools` | ACP 出口 | 外部 Agent 读知识/搜 pack 的唯一 CLI 契约 |
| CodeGraph / RTK | 可选 CLI | `~/.terrain/bin/` 或 bunx 降级 |
| `.terrain/agent/repomix.md` | 本地索引 | 不入库；grep/read 按需，禁止整包加载 |
| 桌面 IPC | 信任边界 | Tauri capabilities 限制前端可调命令 |
| CI/CD | 调用方 | 可脚本化 `terrain assets register/pack-agent/run-litho` |

**知识消费优先级**（矛盾时）：repomix 源码 > codegraph > `agent/context.md` > `human/`；`freshness_score < 50` 时宏观上下文不可信。

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 知识根路径 | `crates/terrain-core/src/registry.rs` | `{repo}/.terrain` |
| 路径布局 | `crates/terrain-core/src/paths.rs` | `KnowledgePaths` |
| IPC Schema | `crates/terrain-core/src/schema.rs` | 30+ 核心类型 |
| Repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | `pack_strategy: architecture-context` |
| Agent 上下文资产 | `crates/terrain-core/src/assets/agent_context.rs` | 分层读取 macro/meso |
| 保鲜评分 | `crates/terrain-core/src/freshness.rs` | `.terrain/.meta/freshness.json` |
| 项目初始化入口 | `crates/terrain-agent/src/project_init.rs` | `run_project_initialization` |
| Litho 编排 | `crates/terrain-agent/src/litho.rs` | 可恢复流水线 |
| Agent 上下文生成 | `crates/terrain-agent/src/agent_context.rs` | 产出 `context.md` |
| Chat 问答 | `crates/terrain-agent/src/chat/mod.rs` | Native + ACP 双模式 |
| ACP tools CLI | `crates/terrain-cli/src/commands/tools.rs` | grep-pack/read-context 等 |
| Tauri 项目命令 | `src-tauri/src/commands/project.rs` | 初始化、资产状态 |
| 前端 API 封装 | `src/lib/api.ts` | invoke 统一入口 |
| TS 类型导出 | `crates/terrain-ts-export/src/main.rs` | `bun run gen:types` |
| 环境集成 catalog | `env-catalog/catalog.json` | Skills + AGENTS.md 片段 |
| Preset Skills | `preset_skills/` | Litho、Ask、SDD、repomix 等 |