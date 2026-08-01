---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手时代的**工程环境管理平台**，核心理念是 "Terrain prepares the ground so agents don't have to guess where to stand"。将 Git 仓库注册后自动扫描代码结构、生成 C4 架构文档（Litho）、维护双轨知识资产（人类可读 `human/` + Agent 友好 `agent/context.md` 与 `agent/repomix.md` 索引包），并通过 DeepWiki 问答与 SDD 四阶段工作流，让人与外部 Coding Agent 共享同一套知识契约。知识存放在仓库内 `.terrain/`，随 Git 分支流转（"知识跟着代码走"，非中心化数据库）。

- **消费者**：人类开发者（Tauri 桌面应用 + `terrain` CLI）、外部 Coding Agent（经 ACP 子进程 / 环境集成注入）。
- **关键约束**：Rust 为 IPC 唯一真源（ts-rs 生成 TS，禁手改生成物）；repomix 索引包为源码权威；`.terrain/` 为知识资产唯一存放处；Agent 须遵守 `AGENTS.md` 中的知识保鲜规则。

## 架构设计

分层（依赖方向自上而下；workspace 成员见根 `Cargo.toml`）：

| 层 | 容器 | 说明 |
|----|------|------|
| 桌面壳 | **src-tauri** | Tauri v2：commands 命令层、托盘、capabilities/ACL、内置资源（preset_skills、sidecar 二进制、bundled-resources） |
| 编排层 | **terrain-agent** | LLM/Agent 编排：ChatEngine（Native LLM / ACP 子进程双后端）、workflows（ask/sdd/init/quick_refresh）、agent context 生成、工具 schema 与节流、运行时 |
| 核心库 | **terrain-core** | 无 LLM 依赖的领域核心：IPC/schema 类型（ts-rs）、注册、ingest 扫描、freshness、资产生成、环境集成、search、sessions、citations、usage |
| CLI | **terrain-cli** | 与 IPC 命令平行的子命令集，供无桌面场景使用 |
| 类型导出 | **terrain-ts-export** | `export-ts-types` 二进制，汇总导出根类型到 `src/lib/generated/` |
| 补丁 crate | **agent-client-protocol-tokio-patched** | 对 ACP tokio 传输的本地补丁（workspace `[patch]`） |
| 前端 | **src/**（Svelte 5） | 面板组件（Ask/Env/Freshness/SDD/Usage/Overview）、stores、`api.ts` invoke 封装、`types.ts` 类型入口 |

**关键依赖**：ADK 家族（adk-acp / adk-model / adk-runner / adk-session / adk-tool）、agent-client-protocol（ACP 协议）、repomix-core（repomix-rs）、ts-rs、tokio。前端依赖 @tauri-apps/api + marked + mermaid + highlight.js + tailwind。

**数据布局**：每仓库 `.terrain/`（`agent/`、`human/`、`knowledge/`、`.meta/`、`.litho-agent/`）；本地 `~/.terrain/registry.json` 仅存项目指针，不含知识正文。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| terrain-core 核心/类型 | 域与 IPC schema（ts-rs）、error、ts_ipc 宏 | crates/terrain-core/src/{schema,ipc,error,ts_ipc}.rs |
| 项目与注册 | 注册/注销、项目元数据、repo 校验、路径解析 | crates/terrain-core/src/{registry,project,repo,path_portable,paths}.rs |
| 扫描/ingest | ProjectScanner（git 元数据 + OpenAPI 导入）、repomix 打包 | crates/terrain-core/src/{ingest,assets/repomix}.rs |
| 资产生成 | agent context、litho、ask 检索、sdd、quick refresh、pack 读取 | crates/terrain-core/src/assets/*.rs |
| Freshness | 保鲜评分、git/codegraph 漂移、台账 | crates/terrain-core/src/freshness/*.rs |
| 环境集成 | skills/工具链/AGENTS.md 片段目录、部署、探针 | crates/terrain-core/src/assets/env/, integrations/ |
| 检索与会话 | KnowledgeSearch、ask/sdd 会话持久化、citations、source 切片 | crates/terrain-core/src/{search,sessions,source,citations}.rs |
| terrain-agent 聊天 | ChatEngine（native/acp）、prompt、工具 schema、节流 | crates/terrain-agent/src/chat/*.rs |
| terrain-agent 工作流 | ask/sdd/init/quick_refresh 编排、context 生成、ACP 运行时 | crates/terrain-agent/src/{workflows,context_generator,runtime,litho,sdd,acp}.rs |
| terrain-cli | 子命令：ask/init/env/knowledge/project/sdd/settings/source/tools/usage/assets | crates/terrain-cli/src/commands/ |
| 桌面命令壳 | Tauri invoke 命令、托盘、内置资源/工具 | src-tauri/src/commands/*.rs, tray.rs, preset_skills.rs, bundled_tools.rs |
| 前端 | 面板 UI、stores、api.ts、markdown/mermaid/usage 工具 | src/lib/{components,stores,api.ts,types.ts} |
| 发行与环境目录 | 跨平台 npm 二进制 + shims；Agent 环境 catalog | npm/, packages/, env-catalog/ |

## 核心流程

1. **项目初始化与扫描**：注册仓库（`initialize_project_cmd`）→ ProjectScanner 采集 git 元数据 + 导入 OpenAPI → 生成 repomix 索引包 `agent/repomix.md` → 生成 `agent/context.md` → freshness 基线 → 环境集成计划/应用（skills、工具链、AGENTS.md 片段）。增量更新走 `run_quick_refresh_cmd`。
2. **DeepWiki Ask**：用户提问（`ask_knowledge_cmd`）→ ChatEngine.ask → 若 agent 资产落后 HEAD 先 `prepare_agent_assets_for_ask` → 三层检索（Macro 预载 context.md → Meso 检索 human/knowledge → Micro grep/read repomix 包）→ 组装带来源引用的回答 → Channel 流式事件（thinking / tool_calls / phase / usage）→ 会话保存。
3. **SDD 四阶段工作流**：创建会话 → 1.Requirements → 2.TechDesign（两者走 Native LLM）→ 3.CodeGen（委托 ACP Agent 直接修改仓库）→ 4.CodeReview；每阶段产出可审查 Markdown 并持久化（`run_sdd_phase_cmd`）。
4. **Litho C4 文档生成**：plan（四阶段：预处理 → C4 研究 → 编排 → 输出）→ 执行生成（LLM）→ 研究产物存 `.terrain/.litho-agent/` 支持中断恢复 → 产出六份 `human/` 文档（含 Mermaid）。入口 `plan_litho_cmd` / `run_litho_generation_cmd`。
5. **Freshness 保鲜**：compute（git snapshot + drift factors + codegraph drift 独立交叉验证）→ 写回 `.terrain/.meta/freshness.json` 台账 → Agent 据此决定信任层级（`compute_freshness_cmd`）。

## 技术选型

- **Rust**：edition 2024（rust-version 1.94）、tokio 全特性 async；workspace `[patch]` 机制挂 ACP tokio 补丁。
- **桌面**：Tauri v2 + 插件 dialog/shell；capabilities/ACL 权限；Channel 流式 IPC；托盘应用。
- **前端**：Svelte 5 + Vite 8 + TypeScript 5 + Tailwind 4；marked / mermaid / highlight.js / html2canvas。
- **LLM/Agent**：ADK（adk-acp/adk-model/adk-runner/adk-session/adk-tool）+ agent-client-protocol；Provider 覆盖 OpenAI 兼容 / LM Studio / Ollama / ACP 子进程（opencode）。
- **源码索引**：repomix-core（repomix-rs，repomix 的 Rust 实现）→ `agent/repomix.md`。
- **类型单向导出**：ts-rs（schema/bindings）→ `src/lib/generated/`，命令 `bun run gen:types`（等价 `cargo run -p terrain-ts-export --bin export-ts-types`）。
- **发行**：npm 平台包（cli/rtk + darwin-arm64 / win32-x64）+ shims（`npm/scripts/write-shims.mjs`）；Windows 交叉构建 `scripts/cross-windows-terrain.sh`。

## 系统边界

- **外部 LLM/Agent**：OpenAI 兼容 API、LM Studio、Ollama（HTTP）；ACP Agent 以子进程运行，SDD CodeGen 阶段被授予仓库修改权限（信任边界）。
- **外部输入**：被注册的 Git 仓库（只读扫描 + 可选写回）、OpenAPI 规范导入、DeepWiki 外部问答面板。
- **存储**：仓库内 `.terrain/`（版本化知识，随分支流转）；`~/.terrain/registry.json`（仅项目指针）；`.terrain/.meta/`（freshness 台账等缓存）；`.terrain/.litho-agent/`（Litho 研究产物）。
- **信任与安全**：tauri capabilities/ACL 约束前端命令；登录 shell PATH 增强（`augment_path_from_login_shell`）；内置 sidecar 二进制（codegraph / rtk / terrain）经 npm 平台包分发；Agent 工具清单 `.terrain/env/agent-tools.json`（不入库）。
- **契约**：Rust schema 为唯一真源，前端禁手改 `src/lib/generated/`；`Option<T>` 生成结果为 `T | null`（非 undefined）；`types.client.ts` 承载 UI 专用扩展类型。

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| IPC/状态 schema | crates/terrain-core/src/schema/, crates/terrain-agent/src/chat/types.rs | ts-rs 注解源 |
| TS 导出二进制 | crates/terrain-ts-export/src/main.rs | gen:types |
| 前端类型入口 | src/lib/types.ts, src/lib/types.client.ts | re-export |
| IPC 命令壳 | src-tauri/src/commands/*.rs | invoke 目标 |
| 前端 API 封装 | src/lib/api.ts | invoke wrapper |
| 注册/扫描 | crates/terrain-core/src/{registry,ingest,repo}.rs | ProjectScanner |
| repomix 打包 | crates/terrain-core/src/assets/repomix.rs | agent/repomix.md |
| agent context 生成 | crates/terrain-core/src/assets/agent_context.rs, crates/terrain-agent/src/context_generator.rs | 本流程产出 |
| Litho 生成 | crates/terrain-core/src/assets/litho.rs, crates/terrain-agent/src/litho.rs | C4 文档 |
| Freshness | crates/terrain-core/src/freshness/*.rs | 保鲜评分 |
| 环境集成 | crates/terrain-core/src/assets/env/, crates/terrain-core/src/integrations/ | skills/toolchain/AGENTS.md |
| ChatEngine | crates/terrain-agent/src/chat/{mod,native,acp,prompt,types}.rs | 双后端 |
| SDD 工作流 | crates/terrain-agent/src/workflows/sdd.rs, crates/terrain-core/src/assets/sdd.rs | 四阶段 |
| Ask/检索 | crates/terrain-agent/src/workflows/ask.rs, crates/terrain-core/src/{assets/ask,search}.rs | 三层检索 |
| CLI 子命令 | crates/terrain-cli/src/commands/ | 与 IPC 平行 |