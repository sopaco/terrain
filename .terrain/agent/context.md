---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览
Terrain 是面向 AI 编码助手时代的工程环境管理平台。它通过自动扫描代码库、生成 C4 架构文档（Litho）、维护双轨知识资产（人类 `human/` + 机器 `agent/`），并通过 DeepWiki 问答与 SDD 工作流，让人类开发者与 Coding Agent 共享知识契约。知识存储在仓库 `.terrain/` 目录，随 Git 流转。

## 架构设计

### Containers
- **terrain-cli**: 命令行入口
- **terrain-core**: 核心业务逻辑
- **terrain-agent**: Agent 运行时与上下文生成
- **terrain-ts-export**: TypeScript 绑定
- **src-tauri**: 桌面应用 GUI
- **preset_skills**: 内置技能模板

### Layers
- **CLI Layer**: `terrain-cli` 指令解析与调度
- **Core Layer**: `terrain-core` 扫描、打包、检索、SDD、Litho
- **Agent Layer**: `terrain-agent` 对话、工具调用、上下文管理
- **Desktop GUI**: Tauri 壳 + Svelte 前端
- **Skills Layer**: 可插拔技能系统

### Key Dependencies
- Rust (Cargo 工作空间)
- Node.js (TypeScript 工具链)
- Tauri (桌面壳)
- Svelte (UI 组件)
- repomix-rs (源码打包)
- Mermaid (图表渲染)

## 模块地图

| Module | Responsibility | Primary paths |
|--------|----------------|---------------|
| `terrain-core` | 核心: 扫描、打包、检索、环境、SDD、Litho | `crates/terrain-core/src/` |
| `terrain-agent` | Agent 运行时: 对话、工具、上下文生成、ACP | `crates/terrain-agent/src/` |
| `terrain-cli` | CLI 命令分发与主入口 | `crates/terrain-cli/src/` |
| `terrain-ts-export` | TypeScript 类型与 IPC 绑定 | `crates/terrain-ts-export/src/` |
| `src-tauri` | 桌面 GUI (Tauri + Svelte) | `src-tauri/src/`, `src/` |
| `preset_skills` | 内置技能: ask, context, architecture, litho, sdd | `preset_skills/{skill}/` |
| `packages/codegraph` | 代码图数据库与查询 | `packages/codegraph/` |
| `env-catalog` | 环境与技能元数据 | `env-catalog/` |

## 核心流程

1. **仓库接入流水线**: 用户注册 Git 仓库 → `ProjectScanner` 采集元数据、解析 OpenAPI → repomix 打包生成 `agent/repomix.md` → 自动生成 C4 文档（Litho 四阶段）→ 刷新知识双轨资产。
2. **DeepWiki 问答**: 用户提问 → `ChatEngine` 检索 Macro 预载上下文 → 必要时 Meso 检索 `human/knowledge/` → Micro `grep_agent_pack` 精读 → 集成 LLM 生成带引用的回答。
3. **SDD 标准化开发**: 选择需求/设计/实现/审查阶段 → 每阶段产出 Markdown 里程碑 → 轻量阶段走 Native LLM，代码实现委托 ACP Agent → 结果回写仓库。
4. **Agent 环境集成**: 检测宿主环境 → 安装 Skills、CLI 工具链、`AGENTS.md` 片段 → 外部 Agent 通过 TCP/Stdio (ACP) 接入 Terrain 知识层。

## 技术选型

- **核心**: Rust (Cargo 工作空间)
- **CLI & 工具链**: Rust (clap)
- **桌面**: Tauri (Rust 壳) + Svelte 5 + TypeScript
- **知识检索**: repomix-rs (源码压缩包)
- **架构生成**: Litho 四阶段流水线 (Mermaid 输出)
- **Agent 协议**: ACP (Agent Client Protocol) + Native LLM
- **代码图**: SQLite (codegraph.db)
- **构建**: bun, vite, cargo

## 系统边界

### 外部依赖与信任边界
| 边界 | 方向 | 说明 |
|------|------|------|
| Git 仓库 | 只读访问 | 扫描元数据、OpenAPI、历史 |
| LLM 提供商 | 网络出站 | Native LLM 与 ACP Agent 推理 |
| 外部 Coding Agent | 本地/远程 | ACP 子进程或 TCP 连接 |
| Skills 市场 | 网络出站 | 安装第三方 Skills |
| 用户文件系统 | 读写 | `.terrain/` 目录与仓库根 |

### 内部数据流
- 源码 → `agent/repomix.md` (压缩包)
- 知识库 → `agent/context.md` (摘要) + `human/` (全文)
- Litho 产物 → `.terrain/.litho-agent/` (中间态) → human 文档

## 代码映射索引

| Concept | Location | Notes |
|---------|----------|-------|
| Core lib | `crates/terrain-core/src/lib.rs` | 公共模块导出 |
| Scan/Pack | `crates/terrain-core/src/repomix.rs` | repomix 集成 |
| Agent context | `crates/terrain-core/src/agent_context.rs` | 上下文生成 |
| Litho pipeline | `crates/terrain-core/src/litho.rs` | C4 文档生成 |
| Agent runtime | `crates/terrain-agent/src/lib.rs` | Agent 主入口 |
| Chat engine | `crates/terrain-agent/src/chat/` | 双后端对话 |
| CLI commands | `crates/terrain-cli/src/commands/` | 子命令拆解 |
| Tauri commands | `src-tauri/src/commands/` | 桌面 IPC |
| Svelte UI | `src/lib/components/` | 所有视图 |
| Ask view | `src/lib/AskBar.svelte` | 问答主入口 |
| Codegraph | `packages/codegraph/src/main.ts` | 代码图 CLI |
| Skills meta | `agent-tools.template.json` | 全局技能配置 |
| Env planner | `crates/terrain-core/src/env/` | 环境探测与规划 |
| Freshness scoring | `crates/terrain-core/src/freshness/` | 知识过期评分 |
| Ingest modules | `crates/terrain-core/src/ingest/` | Git/OpenAPI 导入 |