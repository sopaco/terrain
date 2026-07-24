---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手时代的**工程环境管理平台**（"Agent 地形系统"）：为 Git 仓库自动扫描结构、生成双轨知识资产（人类 Litho C4 文档 + 机器 `agent/` 压缩上下文）、提供 DeepWiki 问答与 SDD 四阶段工作流，并向外部 Coding Agent 注入 Skills / 工具链 / `AGENTS.md` 契约。知识存放在仓库 `.terrain/`，随分支流转；项目登记在本地 `~/.terrain/registry.json`。消费者：人类开发者（Tauri 桌面 + CLI）、外部 Agent（`terrain tools` / ACP）。核心约束：Rust 为 IPC 真源、知识分层检索（Macro→Meso→Micro）、无中心化数据库。

## 架构设计

| 容器 | 职责 | 主要路径 |
|------|------|----------|
| **Tauri Shell** | 桌面宿主、IPC 桥、系统托盘、捆绑 sidecar | `src-tauri/` |
| **Svelte UI** | 项目总览、DeepWiki、SDD、Env 集成、用量面板 | `src/` |
| **terrain-core** | 领域逻辑：项目注册、资产管线、保鲜、检索、Schema | `crates/terrain-core/` |
| **terrain-agent** | LLM/ACP 编排：Chat、Litho、SDD、上下文生成 | `crates/terrain-agent/` |
| **terrain-cli** | 无头 CLI：`init`/`ask`/`sdd`/`tools` 等 | `crates/terrain-cli/` |
| **知识资产层** | 仓库内持久化知识（随 Git） | `.terrain/` |
| **Env Catalog** | Skills、AGENTS.md 片段、工具清单模板 | `env-catalog/`、`preset_skills/` |
| **NPM 分发** | 跨平台 CLI/RTK 二进制 shim | `npm/packages/` |

**依赖方向**：UI/CLI → terrain-agent → terrain-core → 文件系统 / Git / repomix-core。ACP 子进程与 Native LLM 为可替换后端，经 `ChatEngine` 统一调度。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| **项目注册与扫描** | 登记仓库、`ProjectScanner` 采集 Git/OpenAPI、触发 repomix 打包 | `crates/terrain-core/src/project.rs`、`ingest/`、`registry.rs` |
| **Agent 资产管线** | 生成/读写 `context.md`、`repomix.md`、Litho/SDD 产物 | `crates/terrain-core/src/assets/` |
| **知识分层** | Macro/Meso/Micro 切片、`context_layers` 分段与按需提取 | `crates/terrain-core/src/assets/context_layers.rs` |
| **保鲜评分** | Git/CodeGraph 漂移检测、`freshness.json` 缓存 | `crates/terrain-core/src/freshness/` |
| **ChatEngine** | DeepWiki 问答；Native LLM 与 ACP 双后端、工具调用 | `crates/terrain-agent/src/chat/` |
| **Litho 生成** | 四阶段 C4 人类文档流水线 | `crates/terrain-agent/src/litho.rs`、`workflows/`、`preset_skills/litho-documents-skill/` |
| **SDD 工作流** | 需求→设计→代码生成→审查四阶段 | `crates/terrain-agent/src/sdd.rs`、`workflows/sdd.rs` |
| **上下文生成** | ACP/Native 架构 `context.md` 合成 | `crates/terrain-agent/src/context_generator.rs`、`agent_context.rs` |
| **环境集成** | Skills/RTK/CodeGraph/`AGENTS.md` 部署与探测 | `crates/terrain-core/src/integrations/`、`env-catalog/` |
| **IPC Schema** | Rust↔TS 载荷定义、ts-rs 导出 | `crates/terrain-core/src/schema/`、`ipc/`、`terrain-ts-export/` |
| **Tauri Commands** | 前端可调用的工作流入口 | `src-tauri/src/commands/` |
| **前端 API 层** | `invoke` 封装、状态 Store | `src/lib/api.ts`、`src/lib/stores/` |

## 核心流程

### 1. 项目初始化（Scan → Pack → Generate）

1. 用户通过 UI 或 `terrain init` 指定 Git 仓库路径与 slug
2. `ProjectScanner` 采集 Git 元数据、可选 OpenAPI，写入 `ScanReport`
3. `pack_agent_assets` 调用 repomix-core 生成 `.terrain/agent/repomix.md`（architecture-context 策略）
4. 并行/串行触发 `context.md`（架构层）与 Litho（人类层）生成；进度经 IPC 事件推送 UI
5. 项目元信息登记至 `~/.terrain/registry.json`，知识正文留在仓库 `.terrain/`

### 2. DeepWiki 问答（三层检索）

1. **Macro**：预载 `agent/context.md` 的概览/架构/模块地图段（`build_context_overview`）
2. **Meso**：按需搜索 `.terrain/human/`、`.terrain/knowledge/`（`search` 模块）
3. **Micro**：`grep_repomix_pack` / `read_agent_pack_file` 切片读源码索引
4. `ChatEngine` 选择 Native LLM 或 ACP 后端执行多轮推理，附 `SourceCitation` 与工具轨迹

### 3. Litho C4 文档生成

1. 预处理：收集目录结构、人类文档片段、已有研究产物（`.terrain/.litho-agent/`）
2. 研究阶段：多轮 LLM 调研写入中间 Markdown
3. 编排阶段：按模板合成六份标准人类文档至 `.terrain/human/`
4. 支持中断恢复；完成后更新保鲜 ledger

### 4. SDD 标准化开发

1. 阶段 1–2（需求、技术设计）：Native LLM 产出 Markdown
2. 阶段 3（代码生成）：委托 ACP Agent 修改仓库
3. 阶段 4（代码审查）：Native LLM 审查
4. 会话产物存 `~/.terrain/sdd/{project}/sessions/{id}/outputs/`（本地、不入库）

## 技术选型

- **语言**：Rust（核心）、TypeScript/Svelte 5（UI）、Bun（前端构建与脚本）
- **桌面**：Tauri 2（`src-tauri/`），sidecar 捆绑 `terrain`/`codegraph`/`rtk` 二进制
- **源码索引**：repomix-core 2.x（Rust 版 repomix，`architecture-context` 打包策略）
- **LLM**：async-openai（Native 后端，OpenAI 兼容 API）
- **ACP**：agent-client-protocol + 本地 patched `agent-client-protocol-tokio`
- **类型桥接**：ts-rs + `terrain-ts-export` → `src/lib/generated/`（禁止手改）
- **Markdown**：pulldown-cmark 渲染；Mermaid 图表（Litho 人类文档）
- **CLI 分发**：npm workspace（`@terrain-ai/cli`、`@terrain-ai/rtk`）+ 平台二进制包
- **关系分析**：CodeGraph CLI（可选，`.codegraph/codegraph.db`）
- **Shell 压缩**：RTK（冗长命令输出截断）

## 系统边界

| 边界 | 类型 | 说明 |
|------|------|------|
| **Git 仓库** | 信任输入 | 扫描源；`.terrain/` 知识随分支版本化 |
| **`~/.terrain/`** | 本地状态 | `registry.json`、SDD 会话、`bin/` 工具链、Settings |
| **LLM Provider** | 外部 API | OpenAI 兼容端点；需用户配置 API Key/Base URL |
| **ACP Agent** | 外部子进程 | 上下文生成、SDD 代码生成、可选 Ask 后端；需配置 binary/command |
| **CodeGraph** | 可选外部工具 | 符号关系/调用链；`bunx codegraph` 降级 |
| **RTK** | 可选外部工具 | Shell 输出治理；`bunx @terrain-ai/rtk` 降级 |
| **OpenAPI 规范** | 可选导入 | `ingest/openapi.rs` 补充 API 边界信息 |
| **无中心化 DB** | 架构约束 | 知识即文件；保鲜为本地 JSON 缓存 |

**信任分界**：宏观架构以 `agent/context.md` 为契约，但 `freshness_score < 70` 须交叉验证 repomix；` < 50` 时以 repomix 源码切片为准。

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 项目扫描器 | `crates/terrain-core/src/ingest/` | `ProjectScanner`、`ScanReport` |
| Repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | 生成 `agent/repomix.md` |
| Agent 上下文 | `crates/terrain-core/src/assets/agent_context.rs` | 读写/保鲜 `context.md` |
| 上下文分层 | `crates/terrain-core/src/assets/context_layers.rs` | Macro/Meso 切片逻辑 |
| 全文检索 | `crates/terrain-core/src/search.rs` | human/knowledge 搜索 |
| Chat 引擎 | `crates/terrain-agent/src/chat/mod.rs` | `ChatEngine`、Native/ACP 后端 |
| Ask 工作流 | `crates/terrain-agent/src/workflows/ask.rs` | DeepWiki 编排 |
| Litho 工作流 | `crates/terrain-agent/src/litho.rs` | C4 四阶段生成 |
| SDD 工作流 | `crates/terrain-agent/src/sdd.rs` | 四阶段 SDD |
| 上下文生成器 | `crates/terrain-agent/src/context_generator.rs` | ACP 模式 architecture context |
| CLI 入口 | `crates/terrain-cli/src/main.rs`、`cli.rs` | 子命令路由 |
| Tauri IPC | `src-tauri/src/commands/` | `scan_project`、`run_litho_generation_cmd` 等 |
| 前端 API | `src/lib/api.ts` | `invoke` 封装 |
| DeepWiki UI | `src/lib/components/DeepWikiPanel.svelte` | 问答面板 |
| TS 类型导出 | `crates/terrain-ts-export/src/main.rs` | `bun run gen:types` |
| Preset Skills | `preset_skills/` | ask/arch/litho/sdd 技能定义 |
| 领域术语 | `.terrain/knowledge/00-glossary.md` | terrain/repomix/Sopaco 等 |