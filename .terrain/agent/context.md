---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手时代的**工程环境管理平台**：为 Git 仓库自动构建「有地图、有路标、有规范」的知识领地，让人类开发者与外部 Coding Agent 共享同一套知识契约。核心消费者为桌面应用用户、CLI 用户、以及通过 ACP 接入的外部 Agent。知识存放在仓库内 `.terrain/`（随 Git 流转），项目登记在本地 `~/.terrain/registry.json`（仅存路径）。关键约束：Rust 为 IPC 类型唯一真源；知识检索遵循 Macro→Meso→Micro 三层模型；`freshness_score < 50` 时宏观上下文不可信，须以 repomix 源码切片为准。

## 架构设计

| 容器 | 职责 | 主要路径 |
|------|------|----------|
| **桌面壳** | Tauri IPC 桥接、系统托盘、捆绑二进制 | `src-tauri/` |
| **前端 UI** | Svelte 5 单页应用：项目总览、DeepWiki、SDD、环境集成 | `src/` |
| **核心域** | 路径、扫描、资产、保鲜、会话、IPC 类型、环境集成 | `crates/terrain-core/` |
| **Agent 运行时** | ChatEngine（Native LLM / ACP）、工作流编排 | `crates/terrain-agent/` |
| **CLI** | 无头命令：ask、sdd、tools、env、assets | `crates/terrain-cli/` |
| **类型导出** | Rust→TypeScript（ts-rs） | `crates/terrain-ts-export/` |
| **ACP 适配** | Agent Client Protocol 子进程通信 | `crates/agent-client-protocol-tokio-patched/` |

**分层依赖**：`src/` → Tauri commands → `terrain-agent` → `terrain-core` ← `terrain-cli`。前端通过 `invoke()` 调用 Rust；CLI 与桌面共享 core/agent 逻辑。

**知识资产布局**（`.terrain/` 内）：
- `agent/context.md` — 架构宏观上下文（本文件）
- `agent/repomix.md` — repomix-core 压缩源码索引
- `human/` — Litho 生成的 C4 人类文档
- `knowledge/` — 私域术语与规范
- `.meta/freshness.json` — 知识保鲜评分缓存

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| terrain-core | 领域核心：路径解析、项目注册、文档读写、repomix 打包、保鲜评分、环境探测 | `crates/terrain-core/src/` |
| terrain-agent | ChatEngine、Ask/SDD/Litho/Init 工作流、ACP 子进程管理 | `crates/terrain-agent/src/` |
| terrain-cli | 命令行入口与子命令路由 | `crates/terrain-cli/src/` |
| terrain-ts-export | IPC 类型批量导出到 `src/lib/generated/` | `crates/terrain-ts-export/src/main.rs` |
| src-tauri | Tauri 应用壳、IPC command 注册、捆绑工具部署 | `src-tauri/src/` |
| 前端 UI | 面板组件、状态 store、API 封装 | `src/lib/` |
| preset_skills | Litho/Ask/SDD/架构生成等内置 Skill | `preset_skills/` |
| env-catalog | Skills 与 AGENTS.md 片段模板、工具目录 | `env-catalog/` |
| npm 分发 | CLI/RTK 平台 shim 与二进制打包 | `npm/` |
| ingest | Git 元数据采集、OpenAPI 导入、ProjectScanner | `crates/terrain-core/src/ingest/` |
| freshness | 知识保鲜：git 漂移、CodeGraph 过期、评分 | `crates/terrain-core/src/freshness/` |
| integrations | 捆绑工具部署、环境集成计划与执行 | `crates/terrain-core/src/integrations/` |

## 核心流程

### 1. 项目初始化与扫描
1. 用户注册 Git 仓库 → 写入 `~/.terrain/registry.json`
2. `ProjectScanner` 采集 Git 元数据、导入 OpenAPI
3. `pack_agent_assets` 调用 repomix-core 生成 `agent/repomix.md`
4. `context_generator` 基于 meta + repomix 生成 `agent/context.md`
5. 可选触发 Litho 四阶段流水线产出 `human/` 文档

### 2. DeepWiki 知识问答（Ask）
1. 前端/CLI 创建 Ask 会话（存于 `~/.terrain/ask/{slug}/`）
2. `ChatEngine` 按配置选择 Native LLM 或 ACP 子进程后端
3. 三层检索注入上下文：Macro 预载 context.md → Meso 搜索 human/knowledge → Micro grep/read repomix
4. 流式返回答案并附 `SourceCitation` 来源引用
5. 消息持久化到会话目录

### 3. Litho C4 文档生成
1. 预处理：扫描仓库结构、收集 terrain-meta 输入
2. C4 研究：多轮 LLM 研究，产物存 `.terrain/.litho-agent/`
3. 编排：按模板合成六份标准人类文档
4. 输出：写入 `.terrain/human/`，含 Mermaid 图表

### 4. SDD 标准化开发
1. 创建 SDD 会话（存于 `~/.terrain/sdd/{slug}/`）
2. Phase 1 需求 → `1.requirements.md`（Native LLM）
3. Phase 2 技术设计 → `2.tech-design.md`（Native LLM）
4. Phase 3 代码生成 → `3.implementation.md` + 仓库变更（ACP Agent）
5. Phase 4 代码审查 → `4.code-review.md`（Native LLM）

## 技术选型

- **语言**：Rust（后端/CLI）、TypeScript（前端）
- **桌面框架**：Tauri 2（`src-tauri/`）
- **前端**：Svelte 5 + Vite + Bun
- **IPC 类型**：ts-rs 单向导出（Rust → `src/lib/generated/`）
- **源码索引**：repomix-core（`architecture-context` 策略）
- **LLM 后端**：OpenAI 兼容 API、Ollama、LM Studio（Native）；ACP 子进程（外部 Agent）
- **Agent 工具链**：CodeGraph（符号关系）、RTK（shell 输出压缩），捆绑于 `packages/`
- **会话存储**：本地文件系统（`~/.terrain/ask/`、`~/.terrain/sdd/`）
- **构建**：Cargo workspace + `bun run gen:types` + Vite

## 系统边界

| 边界 | 说明 | 信任级别 |
|------|------|----------|
| **仓库内 `.terrain/`** | 知识资产随 Git 协作；agent/human/knowledge 分层 | 高（版本化） |
| **`~/.terrain/registry.json`** | 项目路径登记，不含知识正文 | 本地可信 |
| **`~/.terrain/bin/`** | 部署的 terrain/codegraph/rtk CLI | 本地可信 |
| **LLM Provider** | OpenAI/Ollama/LM Studio 等外部 API | 需配置 API Key |
| **ACP Agent** | 外部 Coding Agent 子进程（Cursor 等） | 用户配置 binary/command |
| **CodeGraph** | 本地符号索引（`.codegraph/codegraph.db`） | 可能过期，须 freshness 交叉验证 |
| **Git** | 元数据采集、漂移检测、HEAD 基线 | 只读 |
| **bundled 二进制** | `src-tauri/binaries/` 内 terrain-cli、rtk | 应用内置 |

**无中心化数据库**；知识跟着代码走。Ask/SDD 会话状态存本地 registry 目录，不入库。

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 知识路径解析 | `crates/terrain-core/src/paths.rs` | `KnowledgePaths` 统一路径 API |
| 项目注册表 | `crates/terrain-core/src/registry.rs` | `register_project`、`knowledge_root_for_repo` |
| 源码打包 | `crates/terrain-core/src/assets/repomix.rs` | repomix-core 封装 |
| 架构上下文生成 | `crates/terrain-agent/src/context_generator.rs` | ACP/Native 双模式 |
| 上下文分层 | `crates/terrain-core/src/assets/context_layers.rs` | Macro/Meso 切片与按需读取 |
| 知识保鲜 | `crates/terrain-core/src/freshness/` | 评分、漂移因子、CodeGraph 过期检测 |
| ChatEngine | `crates/terrain-agent/src/chat/mod.rs` | Native + ACP 双后端 |
| Ask 工作流 | `crates/terrain-agent/src/workflows/ask.rs` | 三层检索编排 |
| SDD 工作流 | `crates/terrain-agent/src/workflows/sdd.rs` | 四阶段状态机 |
| Litho 生成 | `crates/terrain-agent/src/litho.rs` | 四阶段 C4 流水线 |
| Tauri IPC 命令 | `src-tauri/src/commands/` | 前端 `invoke()` 入口 |
| 前端 API 层 | `src/lib/api.ts` | IPC 封装与类型导入 |
| IPC 类型定义 | `crates/terrain-core/src/ipc/` | Rust 侧唯一真源 |
| TS 类型生成 | `crates/terrain-ts-export/src/main.rs` | `bun run gen:types` |
| CLI 入口 | `crates/terrain-cli/src/main.rs` | 子命令路由 |
| CLI tools 子命令 | `crates/terrain-cli/src/commands/tools.rs` | ACP 模式 grep-pack/read-pack-file |
| 环境集成 | `crates/terrain-core/src/assets/env/` | Skills/AGENTS.md 部署 |
| 设置与 LLM 配置 | `crates/terrain-core/src/settings.rs` | `ModelSettings`、`AcpSettings` |