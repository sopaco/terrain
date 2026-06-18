---
type: agent_context
project: mind-mesh
title: Agent Architecture Context
source: /Users/bjsttlp485/Workspace/SAW/mind-mesh
---

## 项目概览

MindMesh 是面向 AI 编码助手的**工程环境管理平台**：扫描 Git 仓库、生成 C4 架构文档、维护结构化知识资产（`.mind-mesh/`），并通过 CLI/桌面应用与 ACP 工具集供人类与外部 Agent 查询。核心约束：**知识原位**（随仓库分支流转，非中心化 DB）、**双轨制输出**（`human/` 叙述性文档 + `agent/` 结构化上下文/repomix）、**可恢复流水线**（Litho/SDD 中间产物持久化）。消费者：开发者（Tauri UI/CLI）、AI 编码助手（`mind-mesh tools` JSON 接口）、CI/CD 自动化。

## 架构设计

**分层容器**

| 层 | 容器 | 职责 | 依赖 |
|----|------|------|------|
| 界面 | `mind-mesh-cli`、`src-tauri`+`src/` | CLI 命令、Tauri IPC、Svelte 桌面 UI | core + agent |
| AI 编排 | `mind-mesh-agent` | Chat/Litho/SDD/上下文生成、ACP 代理通信 | core、LLM、ACP |
| 基础设施 | `mind-mesh-core` | 扫描、存储、搜索、schema、freshness、env 集成 | 文件系统、Git |

**知识分层（Ask 消费模型）**

| 层级 | 路径 | 用途 |
|------|------|------|
| Macro | `agent/context.md` §1–3 | 预加载架构概览 |
| Meso | `agent/context.md` §4–7、`human/`、`knowledge/` | 按需 `read-context --section` |
| Micro | `agent/repomix.md` | `grep-pack` / `read-pack-file` 源码切片 |

**数据布局**：`{repo}/.mind-mesh/` 存知识正文；`~/.mind-mesh/registry.json` 仅存 slug→repo 映射；Litho 中间产物在 `.mind-mesh/.litho-agent/`；SDD 会话在 `.mind-mesh/.sdd/`。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| mind-mesh-core | 知识基础设施：路径、schema、文档、搜索、freshness | `crates/mind-mesh-core/src/` |
| assets | Agent 上下文/Litho/SDD 计划、repomix 打包、meta 收集 | `crates/mind-mesh-core/src/assets/` |
| ingest | Git 仓库扫描、OpenAPI 导入、项目索引 | `crates/mind-mesh-core/src/ingest/` |
| mind-mesh-agent | LLM 会话、文档生成编排、ACP 通信 | `crates/mind-mesh-agent/src/` |
| chat | DeepWiki 问答：Native/ACP 双后端、流式、工具调用 | `crates/mind-mesh-agent/src/chat/` |
| litho | C4 文档四阶段流水线（研究→编排→输出） | `crates/mind-mesh-agent/src/litho.rs` |
| project_init | 初始化全流程编排（扫描→打包→Litho→上下文） | `crates/mind-mesh-agent/src/project_init.rs` |
| sdd | 四阶段标准化开发（需求→设计→编码→审查） | `crates/mind-mesh-agent/src/sdd.rs` |
| context_generator | Agent 上下文生成抽象（Native/ACP 实现） | `crates/mind-mesh-agent/src/context_generator.rs` |
| mind-mesh-cli | clap 入口：list/scan/search/tools/assets/env | `crates/mind-mesh-cli/src/main.rs` |
| 桌面 UI | Tauri IPC 命令 + Svelte 面板（Ask/Litho/SDD/Env） | `src-tauri/src/commands/`、`src/` |
| env 集成 | Skills/AGENTS.md/工具链检测与安装 | `crates/mind-mesh-core/src/assets/env/`、`env-catalog/` |

## 核心流程

### 1. 项目初始化
1. 用户触发（UI 或 CLI）→ `run_project_initialization()`
2. `ProjectScanner::scan_repo()` 本地扫描，写入 `index.md`
3. `pack_agent_assets()` 生成 `agent/repomix.md`
4. 若 ACP 可用 → `run_litho_generation()` 产出 `human/` C4 文档
5. `run_agent_context_generation()` 生成 `agent/context.md`
6. 注册到 `~/.mind-mesh/registry.json`，更新 freshness ledger

### 2. Litho 文档生成
1. 检查 `human/` 完整性；若研究产物已就绪则跳过研究
2. `prepare_litho_generation()` 构建 LithoPlan，注入 skill 环境变量
3. ACP Agent 执行：预处理 → C4 研究 → 编排 → 输出
4. 轮询 `.litho-agent/` 与 `human/` 进度（3s 间隔，45min 超时）
5. 编排阶段最多 3 次重试补齐缺失文档

### 3. DeepWiki 问答（ChatEngine）
1. 检查 repomix/context 就绪，缺失则自动触发资产生成
2. 预加载 Macro 上下文 + repomix 元数据构建 prompt
3. Native LLM（`chat/native.rs`）或 ACP Agent（`chat/acp.rs`）执行，可调用 `grep-pack`/`read-context`/`search`
4. 流式推送至 UI；完成后提取 `SourceCitation`

### 4. SDD 工作流
1. 创建/选择 SDD 会话 → `create_sdd_session()`
2. 按序执行四阶段：`run_sdd_phase()`（需求/设计/编码/审查）
3. 每阶段 LLM 或 ACP 生成 Markdown 产物至 `.mind-mesh/.sdd/`
4. 阶段输出可人工审查后继续下一阶段

## 技术选型

- **语言/运行时**：Rust workspace（`mind-mesh-core`、`mind-mesh-agent`、`mind-mesh-cli`、`mind-mesh-ts-export`）、Tokio 异步
- **桌面壳**：Tauri v2（`src-tauri/`），IPC 暴露项目/知识/聊天/SDD/环境命令
- **前端**：Svelte 5 + Vite + TypeScript（`src/`）；IPC 类型由 ts-rs 从 Rust 生成（`bun run gen:types`）
- **CLI**：clap 派生，6 命令组（list/scan/search/tools/assets/env）
- **LLM**：adk-rust / async-openai；支持 OpenAI、Ollama、LM Studio
- **ACP**：OpenCode 兼容代理（`MIND_MESH_ACP_*` 配置）；Litho/SDD/上下文生成可 ACP 模式
- **源码打包**：repomix-core（`architecture-context` 策略，364 文件）
- **包管理**：Cargo workspace + Bun（`packages/` 分发 codegraph/rtk）
- **文档格式**：YAML frontmatter + Markdown；Mermaid 图表
- **搜索**：本地 Markdown 全文搜索（无外部搜索引擎）
- **持久化**：文件系统即数据库；Git 快照驱动 freshness 漂移检测

## 系统边界

| 边界 | 类型 | 交互方式 | 信任/约束 |
|------|------|----------|-----------|
| Git 仓库 | 输入 | 文件系统读取、walkdir 扫描 | 只读源码；知识写入 `{repo}/.mind-mesh/` |
| LLM API | 外部服务 | HTTP（OpenAI/Ollama/LM Studio） | 需 API Key/本地端点；温度默认 0 |
| OpenCode ACP | 外部进程 | 子进程 IPC，`MIND_MESH_ACP_BINARY` | Litho/SDD/编码阶段委托；可 `--non-interactive` |
| `~/.mind-mesh/registry.json` | 本地配置 | JSON 读写 | 仅存路径映射，不含知识正文 |
| `mind-mesh tools` | 对外 API | JSON stdout（ACP 模式） | 供 Cursor/OpenCode 等 Agent 调用 |
| CodeGraph / RTK | 捆绑工具 | `packages/` 二进制 + env 集成 | 可选；通过 `env apply` 部署至 `.agents/skills/` |
| preset_skills | 内置 Skill | 环境变量指向 skill 目录 | Litho/SDD/Agent 上下文生成 prompt 模板 |
| CI/CD | 调用方 | CLI 脚本 | `assets register` → `pack-agent` → `run-litho` |

**离线能力**：扫描、搜索、repomix 打包、freshness 计算不依赖 LLM；Litho/SDD/Chat 需 LLM 或 ACP。

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 知识根路径解析 | `crates/mind-mesh-core/src/registry.rs` | `knowledge_root_for_repo()` → `{repo}/.mind-mesh` |
| 目录布局地图 | `crates/mind-mesh-core/src/paths.rs` | `KnowledgePaths`：human/agent/litho/sdd 路径 |
| 核心类型定义 | `crates/mind-mesh-core/src/schema.rs` | `ProjectMeta`、`LithoPlan`、`SddPhase`、`FreshnessSummary` |
| Repomix 打包 | `crates/mind-mesh-core/src/assets/repomix.rs` | 生成 `agent/repomix.md` |
| Agent 上下文生成 | `crates/mind-mesh-agent/src/agent_context.rs` | 写入 `agent/context.md`，≤14 KiB |
| 上下文生成器 trait | `crates/mind-mesh-agent/src/context_generator.rs` | Native/ACP 可插拔实现 |
| Freshness 漂移 | `crates/mind-mesh-core/src/freshness.rs` | Git HEAD vs 资产生成基线 |
| 全文搜索 | `crates/mind-mesh-core/src/search.rs` | `KnowledgeSearch` |
| IPC 类型导出 | `crates/mind-mesh-ts-export/src/main.rs` | Rust→TS 生成至 `src/lib/generated/` |
| CLI tools 子命令 | `crates/mind-mesh-cli/src/main.rs` | `grep-pack`、`read-pack-file`、`read-context` |
| Tauri IPC | `src-tauri/src/commands/` | chat/litho/sdd/scan 事件流 |
| 前端 API 层 | `src/lib/api.ts` | 调用 Tauri invoke |
| Ask 面板 | `src/lib/components/DeepWikiPanel.svelte` | DeepWiki UI |
| Litho skill | `preset_skills/litho-documents-skill/SKILL.md` | 四阶段 C4 生成 prompt |
| Env catalog | `env-catalog/catalog.json` | Skills/AGENTS.md 集成清单 |