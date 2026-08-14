---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手时代的**工程环境管理平台**（Sopaco 开源，含 repomix-rs）。核心理念："Terrain prepares the ground so agents don't have to guess where to stand"——注册一个 Git 仓库后自动完成代码扫描、源码打包（repomix）、Agent 上下文生成、C4 架构文档（Litho）、双轨知识资产（人类 `human/` + Agent `agent/`），并对外部 Coding Agent 提供 Ask 知识问答与 SDD 四阶段工作流。知识存放于仓库内 `.terrain/`，随 Git 分支流转（"知识跟着代码走"）。消费者：桌面应用（Tauri+Svelte）、CLI（`terrain`/`terrain tools`）、外部 Coding Agent（ACP 子进程）。核心约束：Rust 为 IPC 唯一真源（ts-rs 生成 TS）、上下文硬上限 16 KiB、非确定性生成资产禁自动合并、按包 grep 而非全量读码。

## 架构设计

```
┌──────────────────────────────────────────────────────────────┐
│ 前端 Svelte 5 (src/)                  Tauri 2 壳 (src-tauri/) │
│  Ask/DeepWiki · SDD · Litho · 环境 · 项目 · 用量 · 托盘        │
└───────────────┬──────────────────────────────────────────────┘
                │ invoke + 流式事件（ts-rs 生成类型，Rust 真源）
┌───────────────▼──────────────────────────────────────────────┐
│ terrain-core  领域核心（纯逻辑，无 LLM 执行）                    │
│  assets/资产生成 · query/search 三层检索 · freshness/           │
│  ingest/扫描 · registry/注册 · sessions · ipc+schema 类型       │
└───────────────┬──────────────────────────────────────────────┘
┌───────────────▼──────────────────────────────────────────────┐
│ terrain-agent 执行层                                           │
│  ChatEngine(Native ADK / ACP 双后端) · tools · 上下文生成       │
│  workflows: Ask / Init / SDD / QuickRefresh                    │
└───────────────┬──────────────────────────────────────────────┘
                │ adk-model(openai/ollama) · agent-client-protocol(ACP)
┌───────────────▼───────────────┬───────────────┬──────────────┐
│ repomix-core 打包 · codegraph │ LLM Providers │ 外部 Agent    │
│ (SQLite) · rtk                │ (OpenAI/Ollama)│ opencode      │
└───────────────────────────────┴───────────────┴──────────────┘
```

- **分层**：UI（Svelte）→ IPC 壳（src-tauri commands）→ 核心（terrain-core，领域逻辑/资产，无执行）→ 执行（terrain-agent，LLM/ACP/工具/工作流）。
- **依赖方向**：terrain-agent → terrain-core；src-tauri、terrain-cli → 两者；`[patch.crates-io]` 本地替换 `agent-client-protocol-tokio`。
- **同一核心三入口**：桌面 app、CLI（`terrain-cli` 直连，`tools` 子命令即 Ask 的 ACP 知识层）、npm 包（`cli`/`rtk` 二进制 shim）。
- **构建期类型流**：ts-rs 注解（`ts-export` feature）→ `terrain-ts-export` 二进制 → `src/lib/generated/`（`bun run gen:types`）。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| terrain-core | 领域核心：资产生成、三层检索、freshness、摄取、注册表、IPC 类型 | `crates/terrain-core/src/` |
| ├ assets/ | repomix 打包、agent context、litho/sdd/ask 资产、增量刷新、上下文分层、env 集成 | `crates/terrain-core/src/assets/` |
| ├ freshness · ingest · schema · sessions | git+codegraph 保鲜评分、ProjectScanner/OpenAPI、ts-rs 结构体、会话持久化 | `crates/terrain-core/src/{freshness,ingest,schema,sessions,ipc}/` |
| terrain-agent | 执行层：ChatEngine、工具 schema/注册、上下文生成、ACP 设置、SDD/Litho 驱动 | `crates/terrain-agent/src/` |
| ├ chat/ | ChatEngine 双后端（native.rs=ADK Runner / acp.rs）、prompt、tracker、types | `crates/terrain-agent/src/chat/` |
| └ workflows/ · runtime | Ask/Init/SDD/QuickRefresh 编排；Runtime 缓存引擎与 ModelConfig | `crates/terrain-agent/src/workflows/`、`runtime.rs`、`acp.rs`、`tools.rs` |
| terrain-cli | CLI 入口：list/scan/init/refresh/search/read + project/settings/ask/sdd/usage/source/tools/assets/env | `crates/terrain-cli/src/` |
| src-tauri | 桌面壳：commands（project/sessions/workflows/knowledge/env/usage/assets/settings）、托盘、bundled tools、preset skills | `src-tauri/src/` |
| src/ (Svelte 5) | UI：Ask/DeepWiki、SDD、Litho、环境面板、用量、项目概览、知识树 | `src/lib/`（components/、stores/、api.ts） |
| terrain-ts-export | 构建期 ts-rs 导出二进制（根类型 → `src/lib/generated/`） | `crates/terrain-ts-export/src/main.rs` |
| env-catalog + preset_skills | 环境注入素材：skills、agents-md 片段、工具 catalog；内置技能（ask/architecture/litho/sdd） | `env-catalog/`、`preset_skills/` |
| npm/ + packages/ + ACP patch | 跨平台 CLI/RTK 二进制 shim 与发布；本地 ACP tokio 修补依赖 | `npm/packages/`、`packages/`、`crates/agent-client-protocol-tokio-patched/` |

## 核心流程

**1. 项目注册 → 知识资产生成**
1. `initialize_project` 将仓库登记到 `~/.terrain/registry.json`（仅路径）。
2. `ProjectScanner` 采集 Git 元数据、按需导入 OpenAPI（`scan_project`）。
3. repomix-core 打包源码 → `.terrain/agent/repomix.md`（`pack_agent_assets`）。
4. LLM 生成 `agent/context.md`（本项目即产物）；可选 Litho 四阶段生成 `human/` C4 文档（预处理→研究→编排→输出，研究产物持久化 `.litho-agent/` 可断点恢复）。
5. freshness 基线记账，后续 git/codegraph 交叉验证漂移。

**2. Ask 知识问答（DeepWiki，三层检索 + 双后端）**
1. Macro：预载 `agent/context.md` 概览/架构/模块地图。
2. Meso：按需 `read_agent_context(section=…)` 或搜索 `human/`、`knowledge/` 文档。
3. Micro：`grep_agent_pack` → `read_agent_pack_file` 查源码切片。
4. `ChatEngine` 执行：Native 后端（ADK Runner，openai/ollama）或 ACP 子进程（opencode，`AcpSettings` 门控 `execution_uses_native_llm` / `execution_uses_acp` / `execution_pure_acp` / `AcpNative`）；LLM 不可用时降级 `fallback_search_reply`。
5. 流式回传 thinking/tool_calls/phase/usage 事件（`AskStreamEvent`）+ 来源引用，会话持久化可选。

**3. SDD 四阶段开发**
1. 需求 Requirements → `1.requirements.md`。
2. 技术设计 TechDesign → `2.tech-design.md`。
3. 代码生成 Codegen → `3.implementation.md` + 仓库改动（委托 ACP Agent）。
4. 代码审查 CodeReview。轻量文档阶段走 Native LLM，代码阶段走 ACP（`run_sdd_phase` 按阶段分发），每阶段产出可审查 Markdown。

**4. 环境集成（Env）**
1. 探测（probe）Skills / CLI 工具 / AGENTS.md 状态（`EnvStatus`）。
2. 规划（plan）差异 → `EnvPlan`/`EnvPlanStep`。
3. 应用（apply）：部署 terrain-knowledge/repomix/codegraph/rtk skills、bundled tools、`AGENTS.md` 片段（`plan_env_integration`/`apply_env_integration`、`deploy_agent_toolchain`）。

## 技术选型

- **Rust**：workspace（terrain-core/terrain-agent/terrain-cli/terrain-ts-export/src-tauri），edition 2024，rust-version 1.94。
- **桌面壳**：Tauri 2（capabilities ACL、plugin-dialog/shell、托盘 + Usage 窗口）。
- **前端**：Svelte 5（runes）+ Vite 8 + Tailwind 4 + marked/mermaid/highlight.js、@lucide/svelte。
- **Ask 分享图片**：离屏挂载真实 `AskShareCard`（复用 MarkdownViewer/markdown.css/mermaid）→ 长文分页 → 原生 canvas 栅格化 PNG；剪贴板经 `copy_image_to_clipboard`、落盘经 `save_png_files`。
- **IPC 类型**：ts-rs 10 + schemars；`bun run gen:types` 生成 `src/lib/generated/`。
- **Agent 运行时**：ADK Rust 1.0 家族（adk-core/agent/runner/session/tool/model `{openai,ollama}`/acp）+ agent-client-protocol 0.11.1（ACP 子进程），本地 `[patch]` tokio 层。
- **源码索引**：repomix-core 2.0（repomix-rs Rust 锈化）打包 `agent/repomix.md`；CodeGraph（SQLite 符号图）做 drift 交叉验证；RTK 压缩 shell 输出。
- **存储**：`.terrain/`（版本化知识）、`~/.terrain/registry.json`（项目指针）、`.codegraph/`（本地索引）。
- **分发**：npm 包（`cli`/`rtk` + darwin-arm64/win32-x64 shims）、`scripts/cross-windows-terrain.sh` 交叉编译、Tauri 打包。
- **基础库**：tokio、serde/serde_json、anyhow/thiserror、tracing、chrono、walkdir/ignore、futures、slug、dotenvy。

## 系统边界

| 边界 | 说明 | 方向 |
|------|------|------|
| Tauri IPC | Rust commands ↔ Svelte（`invoke` + 流式事件），Rust 类型唯一真源 | 内 |
| LLM Providers | OpenAI 兼容 / Ollama（adk-model），Native 轻量阶段（Ask 摘要、SDD 文档） | 出 |
| ACP 子进程 | 外部 Coding Agent（opencode）经 agent-client-protocol 调用（SDD 代码生成/Litho/Ask）；`acp_config_json` 注入配置与 env；可 spawn 任意命令 → 信任边界，经 `AcpSettings` 授权门控 | 出 |
| 本地注册表 | `~/.terrain/registry.json` 仅存项目路径，无知识正文 | 本地 |
| 知识文件系统 | `.terrain/agent/`（生成）、`human/`（生成）、`knowledge/`（人工）、`.litho-agent/`（研究中产物）、`repomix.md`（本机索引） | 本地 |
| 外部代码 | 只读扫描/打包（git 元数据、OpenAPI 导入、repomix）；不写目标仓库（SDD Codegen 除外） | 出 |
| 工具二进制 | CodeGraph / RTK / terrain CLI 随项目分发（`packages/`、`~/.terrain/bin/`、npm shims） | 出 |
| Git | 摄取、freshness 基线比对（baseline HEAD、ledger）、`.gitattributes` 声明生成资产 `-merge` | 出 |

信任边界：前端不信任 Rust 返回值（Rust 校验）；ACP 子进程视为外部实体（spawn 任意命令，须经授权配置）；生成资产非确定性 → 冲突禁手工合并，保留任一版本后重跑 scan 重生成。IPC `Option<T>` → `T | null`，前端判空按此契约；`read-pack-file`/`grep-pack` 是 Agent 唯一源码入口。

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| 资产生成管线 | `crates/terrain-core/src/assets/mod.rs` | repomix/context/litho/sdd/ask/env 聚合 |
| repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | `pack_agent_assets`、pack 新鲜度 |
| 上下文分层/生成 | `crates/terrain-core/src/assets/context_layers.rs`、`agent_context.rs` | macro/meso/on-demand 切片 |
| 增量刷新 | `crates/terrain-core/src/assets/incremental.rs`、`crates/terrain-agent/src/workflows/quick_refresh.rs` | |
| Litho 生成 | `crates/terrain-core/src/assets/litho.rs` + `crates/terrain-agent/src/litho.rs` | 四阶段、`.litho-agent/` 恢复 |
| SDD 工作流 | `crates/terrain-agent/src/workflows/sdd.rs`、`crates/terrain-agent/src/sdd.rs` | 阶段分发 LLM/ACP |
| Ask 检索 | `crates/terrain-core/src/assets/ask.rs`、`crates/terrain-agent/src/workflows/ask.rs` | 三层检索 + fallback |
| ChatEngine 双后端 | `crates/terrain-agent/src/chat/mod.rs`、`native.rs`、`acp.rs` | ADK Runner / ACP |
| 工具 schema/注册 | `crates/terrain-agent/src/tools.rs`、`tool_schema.rs`、`compat_tool.rs`、`tool_session_cache.rs` | AgentToolPaths |
| 运行时/引擎缓存 | `crates/terrain-agent/src/runtime.rs`、`builder.rs` | ModelConfig + AcpSettings |
| 新鲜度 | `crates/terrain-core/src/freshness/` | compute/scoring/git/codegraph/drift_factors/ledger |
| 摄取/注册 | `crates/terrain-core/src/ingest/`（git/openapi）、`registry.rs`、`project.rs` | ProjectScanner/ScanReport |
| 环境集成 | `crates/terrain-core/src/integrations/`、`assets/env/`、`agent_tools_deploy.rs`、`bundled_tools.rs` | EnvPlan/Status、usage 探测 |
| IPC 类型 | `crates/terrain-core/src/schema/`、`ipc/`、`crates/terrain-agent/src/chat/types.rs` | ts-export 注解 |
| Tauri 命令层 | `src-tauri/src/commands/` | project/sessions/workflows/knowledge/env/usage/assets |
| 前端 IPC 封装 | `src/lib/api.ts`、`types.ts`、`types.client.ts` | invoke + 生成类型入口 |
| Ask 分享/长图导出 | `src/lib/askShareImage.ts`、`components/AskShareCard.svelte`、`ShareImageButton.svelte`、`shareExport.ts`、`clipboard.ts`、`src-tauri/src/commands/settings.rs` | 离屏渲染真实 MarkdownViewer、分页栅格化 PNG；复制/导出经 `copy_image_to_clipboard`/`save_png_files` |
| CLI + Ask tools | `crates/terrain-cli/src/cli.rs`、`commands/tools.rs` | `terrain tools` 知识层 |
| ACP 协议 patch | `crates/agent-client-protocol-tokio-patched/src/acp_agent.rs` | Cargo patch |
