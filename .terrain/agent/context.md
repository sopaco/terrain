---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向人类开发者与 AI 编码助手的**工程环境管理平台**：扫描 Git 仓库、生成 C4 架构文档（Litho）、维护 `.terrain/` 结构化知识资产，并通过 DeepWiki 问答与 SDD 工作流支撑 AI 辅助开发。消费者：桌面/CLI 用户、外部 Agent（OpenCode 等，经 ACP 调用 `terrain tools`）。核心约束：知识**原位存储**于仓库 `.terrain/`（非中心 DB）；扫描/搜索可离线；Litho/SDD CodeGen 依赖 ACP Agent；宏观上下文受 freshness 评分约束（<50 时降权）。

## 架构设计

**分层**：`terrain-core`（基础设施，不依赖 Agent）→ `terrain-agent`（LLM/ACP 编排）→ `terrain-cli` / `src-tauri`+`src/`（双通道 UI）。

| 容器 | 技术 | 职责 |
|------|------|------|
| terrain-core | Rust | 扫描、知识路径、文档解析、搜索、repomix 打包、freshness、注册表 |
| terrain-agent | Rust + adk-rust | Chat、Litho、Agent 上下文、项目初始化、SDD、ACP 通信 |
| terrain-cli | Rust (clap) | `scan`/`search`/`assets`/`tools`/`env` 命令组 |
| 桌面应用 | Tauri v2 + Svelte 5 | IPC 暴露核心与 Agent 能力，文档/问答/SDD/环境 UI |
| 知识资产 | 文件系统 | `.terrain/{agent,human,knowledge,.meta}` + `~/.terrain/registry.json` |
| 外部 Agent | ACP 进程 | Litho 全流水线、SDD CodeGen、可选 Ask 执行 |

**主要依赖流向**：UI/CLI → core（读写 `.terrain/`、扫描 repo）→ agent（LLM/ACP）→ 外部 LLM API 与 ACP 二进制。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| 知识资产管理 | Agent 上下文/repomix/Litho·SDD 计划、pack 读写 | `crates/terrain-core/src/assets/` |
| 源码扫描 | Git 仓库扫描、技术栈检测、OpenAPI 导入 | `crates/terrain-core/src/ingest/` |
| 文档与搜索 | YAML+Markdown 解析、全文搜索、引用 | `crates/terrain-core/src/doc.rs`, `search.rs`, `source.rs` |
| 路径与注册 | 知识目录布局、项目注册表 | `crates/terrain-core/src/paths.rs`, `registry.rs` |
| 数据模型 | IPC/领域类型（serde + 可选 ts-rs） | `crates/terrain-core/src/schema.rs` |
| Chat 引擎 | DeepWiki 问答、流式输出、工具调用三层模型 | `crates/terrain-agent/src/chat/` |
| Litho 生成 | C4 文档流水线编排、断点续传 | `crates/terrain-agent/src/litho.rs` |
| Agent 上下文 | 生成 `agent/context.md`（Native/ACP） | `crates/terrain-agent/src/agent_context.rs`, `context_generator.rs` |
| 项目初始化 | 扫描→打包→Litho→上下文 全流程 | `crates/terrain-agent/src/project_init.rs` |
| SDD 工作流 | 需求→设计→编码→审查 四阶段 | `crates/terrain-agent/src/sdd.rs` |
| ACP 协议 | OpenCode 代理通信、工具会话 | `crates/terrain-agent/src/acp.rs`, `crates/agent-client-protocol-tokio-patched/` |
| 桌面 UI | 面板组件、Tauri IPC 桥、状态 | `src/`, `src-tauri/src/commands/` |

## 核心流程

### 1. 项目初始化
1. 用户触发 UI 或 `terrain assets` 相关命令 → `run_project_initialization()`。
2. `ProjectScanner::scan_repo()` 本地扫描，写入项目索引。
3. `pack_agent_assets()` 经 repomix-core 生成 `agent/repomix.md`。
4. 若 ACP 可用：`run_litho_generation()` 产出 `human/` C4 文档。
5. `run_agent_context_generation()` 生成 `agent/context.md` → 返回 `ProjectInitResult`。

### 2. Litho 文档生成
1. 检查 `human/` 完整性；已完整则短路返回。
2. 检查 `.terrain/.litho-agent/` 研究产物；就绪则仅编排，否则全流水线（预处理→C4 研究→编排→输出）。
3. ACP Agent 读取 `preset_skills/litho-documents-skill/`，写入研究与人类文档。
4. 主进程轮询文件进度（约 3s 间隔），超时或稳定后结束；编排最多 3 次重试。

### 3. DeepWiki 问答（知识三层）
1. `ChatEngine.ask()` 检查 repomix/context 就绪，必要时自动补齐资产。
2. **Macro**：预加载 `agent/context.md` 概览；**Meso**：`read-context --section`；**Micro**：`grep-pack` → `read-pack-file`。
3. Native LLM 或 ACP 执行，流式推送至 UI，提取 `SourceCitation`。

### 4. SDD 四阶段
1. 顺序执行 Requirements → TechDesign（Native LLM）→ CodeGen（ACP Agent）→ CodeReview（Native LLM）。
2. 每阶段校验前一阶段输出文件；产物存于 `~/.terrain/sdd/{project}/sessions/{id}/outputs/`。

## 技术选型

- **语言/运行时**：Rust 2024、tokio 异步 IO
- **桌面壳**：Tauri v2（WebView + Rust IPC）
- **前端**：Svelte 5、TypeScript、Vite、TailwindCSS；IPC 类型由 `terrain-ts-export` + ts-rs 从 Rust 生成
- **AI 框架**：adk-rust（adk-agent/runner/session/tool/model）；ACP via `agent-client-protocol`（Windows 窗口补丁在 `agent-client-protocol-tokio-patched`）
- **源码打包**：repomix-core v2（`architecture-context` 策略）
- **CLI 解析**：clap；**发布**：`npm/packages/cli` 平台 shim + `~/.terrain/bin/`
- **辅助工具**：CodeGraph（符号关系）、RTK（shell 输出压缩）；Skills 在 `env-catalog/skills/`
- **存储**：Markdown + YAML frontmatter + JSON 元数据；无服务端 DB

## 系统边界

| 边界 | 类型 | 说明 |
|------|------|------|
| LLM API | 外部 HTTP | OpenAI / Ollama / LM Studio（`OPENAI_*`、`OLLAMA_*`、`LMSTUDIO_*`） |
| ACP Agent | 外部进程 | 默认 `opencode`（`TERRAIN_ACP_BINARY`）；Litho、SDD CodeGen、可选上下文生成 |
| Git 仓库 | 只读输入 | 扫描与 repomix 源；知识写入 `.terrain/` |
| `~/.terrain/registry.json` | 本地信任 | 项目 slug ↔ repo 路径映射，不含知识正文 |
| `terrain tools` | ACP 出口 | JSON stdout：`pack-meta`、`grep-pack`、`read-pack-file`、`read-context`、`search` |
| CodeGraph / RTK | 可选 CLI | `~/.terrain/bin/` 或 `bunx`/`npx` 降级 |
| 新鲜度 | 内部契约 | `.terrain/.meta/freshness.json`；冲突优先级：repomix > CodeGraph > context > human |

**信任边界**：外部 Agent 仅通过 repomix pack 读源码（非 live FS）；LLM/ACP 为不可信推理层，产物需 freshness 与源码交叉验证。

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 项目初始化入口 | `crates/terrain-agent/src/project_init.rs` | `run_project_initialization` |
| Repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | `pack_agent_assets` |
| Agent 上下文生成 | `crates/terrain-agent/src/agent_context.rs` | Native/ACP 双模式 |
| Litho 编排 | `crates/terrain-agent/src/litho.rs` | `run_litho_generation` |
| Chat/Ask | `crates/terrain-agent/src/chat/mod.rs` | `ChatEngine::ask` |
| SDD 阶段执行 | `crates/terrain-agent/src/sdd.rs` | `run_sdd_phase` |
| 知识路径布局 | `crates/terrain-core/src/paths.rs` | `KnowledgePaths` |
| 项目注册 | `crates/terrain-core/src/registry.rs` | `knowledge_root_for_repo` |
| CLI 入口 | `crates/terrain-cli/src/main.rs`, `cli.rs` | 六组命令 |
| ACP 工具子命令 | `crates/terrain-cli/src/commands/tools.rs` | `terrain tools *` |
| Tauri IPC 命令 | `src-tauri/src/commands/` | `chat`, `knowledge`, `assets`, `project`, `sdd`, `env` |
| 前端 API 桥 | `src/lib/api.ts` | `invoke` 封装 |
| IPC 类型导出 | `crates/terrain-ts-export/src/main.rs` | `bun run gen:types` |
| Litho Skill | `preset_skills/litho-documents-skill/` | 四阶段流水线定义 |
| 环境集成目录 | `env-catalog/` | Skills、AGENTS.md 片段、工具模板 |
| Agent 知识输出 | `.terrain/agent/context.md`, `repomix.md` | 本文件与源码索引 |
| 人类 C4 文档 | `.terrain/human/` | Litho 产物 |
| Freshness 账本 | `.terrain/.meta/freshness.json` | 保鲜评分与漂移因子 |