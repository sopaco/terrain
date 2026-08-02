---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手时代的**工程环境管理平台**："Terrain prepares the ground so agents don't have to guess where to stand"。它把一个 Git 仓库注册为项目后，自动扫描代码结构、打包 repomix 索引、生成机器友好的 `agent/context.md` 与 C4 人类文档（Litho），并提供 DeepWiki 问答与 SDD 四阶段开发工作流，让人类与外部 Coding Agent 共享同一套"知识跟着代码走"的知识契约（存于仓库内 `.terrain/`）。消费方：Tauri 桌面应用（Svelte 前端）、`terrain` CLI、npm 分发的二进制。关键约束：Rust 为 IPC 唯一真源（ts-rs 生成 TS，禁手改生成物）；`src/lib/generated/` 与 `agent/repomix.md` 不得手改。

## 架构设计

```
┌─ Svelte 5 前端 (src/) ──────────────────────────────┐
│  panels / stores / api.ts (invoke) ← Tauri IPC      │
└─────────────────────────────────────────────────────┘
                      │ (capabilities: default.json)
┌─ src-tauri/ 桌面壳（命令分发、tray、bundled 资源引导）┐
└─────────────────────────────────────────────────────┘
                      │ terrain-core（无 LLM，纯领域）
┌─ terrain-core: assets / freshness / ingest / ipc+schema / prompts / sessions / registry / search / citations / usage / env ─┐
                      │
┌─ terrain-agent: ChatEngine(native/acp) + workflows(ask/sdd/init/quick_refresh) + litho + agent_context + acp ─┐
                      │
┌─ agent-client-protocol-tokio-patched: ACP 协议客户端（opencode 子进程）─┐
```

- **分层**：Tauri 壳 → `terrain-core`（领域核心、无 LLM 依赖）→ `terrain-agent`（Agent 执行编排）→ ACP 客户端 / LLM HTTP 后端。
- **IPC 单真源**：`terrain-core/src/schema|ipc` 用 ts-rs 导出到各 crate `bindings/`，`terrain-ts-export` 汇总生成 `src/lib/generated/`；前端经 `src/lib/types.ts` 再导出。
- **数据落点**：仓库内 `.terrain/`（随 Git 流转，含 `agent/context.md`、`agent/repomix.md`、`human/`、`knowledge/`、`.meta/`）；本地 `~/.terrain/registry.json` + `registry/`（会话、sdd 输出）+ `~/.terrain/bin/`（sidecar 工具）。
- **执行双后端**：`ChatEngine` 支持 Native LLM（OpenAI 兼容 HTTP）与 ACP 子进程（`opencode`），由 `AgentExecution`/`AcpSettings` 决定（纯 ACP / Native / 混合）。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| Tauri 壳 | 命令分发、窗口/tray、bundled 工具与 preset skills 引导 | `src-tauri/src/` |
| 前端 UI | 面板、状态、ask/sdd/litho 交互、markdown/mermaid 渲染 | `src/lib/` |
| assets | agent context 生成、repomix 打包、litho/sdd 计划、知识查询、env 集成 | `crates/terrain-core/src/assets/` |
| freshness | 知识保鲜评分、ledger、git 快照、codegraph 漂移 | `crates/terrain-core/src/freshness/` |
| ingest | 项目扫描、git 元数据、OpenAPI 导入、repo 遍历 | `crates/terrain-core/src/ingest/` |
| schema / ipc | IPC 类型（ts-export 注解）、聊天/工作流事件流 | `crates/terrain-core/src/{schema,ipc}/` |
| sessions / registry | ask/sdd 会话持久化、项目注册、路径解析 | `crates/terrain-core/src/{sessions,registry.rs,paths.rs,project.rs}` |
| search / source / citations | 知识搜索、源码切片、来源引用 | `crates/terrain-core/src/{search,source,citations}.rs` |
| ChatEngine | Native/ACP 双后端、流式回复、工具调用追踪 | `crates/terrain-agent/src/chat/` |
| workflows | ask、sdd、init、quick_refresh 编排 | `crates/terrain-agent/src/workflows/` |
| 生成器 | Litho C4 文档、agent context 生成、prompt 组装 | `crates/terrain-agent/src/{litho,agent_context,context_generator}.rs` |
| ACP 客户端 | opencode ACP 子进程协议（tokio patch） | `crates/agent-client-protocol-tokio-patched/src/` |
| CLI | 无头命令：list/scan/init/ask/sdd/env/tools/assets | `crates/terrain-cli/src/` |
| TS 导出 | Rust 类型 → TS bindings 汇总 | `crates/terrain-ts-export/src/main.rs` |
| 环境资产 | skills、agents-md 片段、工具目录、npm 分发 | `env-catalog/`、`preset_skills/`、`npm/` |

## 核心流程

**1. 项目注册与扫描（init → scan → assets）**
1. `initialize_project_cmd` 校验仓库、注册到 `~/.terrain/registry.json`、写入 `.terrain/` 骨架。
2. `scan_project` 用 `ProjectScanner` 采集 git 元数据 + OpenAPI 导入，产出 `ScanReport`。
3. `pack_agent_assets_cmd` 调 repomix-core 生成 `agent/repomix.md`（grep 索引包）。
4. `run_agent_context_generation_cmd` 按 `agent-architecture-skill` 生成 `agent/context.md`；freshness 记录 baseline git HEAD。
5. 可选 Litho：`plan_litho_cmd` → 四阶段生成 `human/` 六份 C4 文档。

**2. DeepWiki 问答（ask_knowledge_cmd）**
1. 若资产落后于 HEAD，先 `prepare_agent_assets_for_ask`（重打包/重生成 context）。
2. **三层检索**：Macro 预载 `agent/context.md` 概览 → Meso 按需 `read_agent_context(section)` 或检索 `human/`、`knowledge/` → Micro `grep/read` repomix 包。
3. `ChatEngine` 经 Native LLM 或 ACP 子进程推理，回传 chunk / thinking / tool_calls / phase / usage。
4. 结果提取来源引用与 tool 记录，持久化到 ask 会话，前端流式渲染。

**3. Litho C4 文档生成（litho.rs + litho-documents-skill）**
1. `plan_litho_cmd` 输出 `LithoPlan`（文档清单、Mermaid 需求）。
2. phase1 预处理源码 → phase2 C4 研究（研究产物落 `.terrain/.litho-agent/`）→ phase3 编排 → phase4 输出 6 份人类文档。
3. 中间产物持久化支持中断恢复；完成态写 `litho_status`，human/ 树刷新。

**4. SDD 四阶段开发（workflows/sdd.rs）**
1. `create_sdd_session_cmd` 建会话，`run_sdd_phase_cmd` 依 `SddPhase`（Requirements→TechDesign→CodeGen→CodeReview）推进。
2. 轻量文档阶段走 Native LLM（`run_sdd_llm_phase`），代码生成阶段委托 ACP Agent（`run_sdd_acp_phase`）。
3. 各阶段产物经 `save_sdd_output_cmd` 存 `~/.terrain/registry/sdd/<slug>/sessions/`。

**5. 环境集成（env）**
1. `get_env_status_cmd` 探测 git/skills/agent-tools/agents-md 现状。
2. `plan_env_integration_cmd` 对照 `env-catalog/catalog.json` 出 `EnvPlan`。
3. `run_env_integration_cmd` 部署 skills（`.agents/skills`、`.claude/skills`）、拼接 `AGENTS.md` 片段、安装 `~/.terrain/bin` sidecar（terrain/rtk/codegraph）。

## 技术选型

- Rust workspace + tokio 异步；`rust-toolchain.toml` 固定工具链。
- Tauri 2 桌面壳；`ts-rs` 生成 TS bindings（`gen:types` → `bun run`）。
- 前端 Svelte 5（runes：`$state`/`$derived`）+ TypeScript + Vite；markdown/mermaid/highlight 渲染。
- 聊天后端：OpenAI 兼容 HTTP（native）+ ACP 协议子进程（opencode，`agent-client-protocol` tokio 版）。
- 源码索引：repomix-core（repomix-rs）打包 `agent/repomix.md`。
- CLI：clap；`terrain` / `rtk` 经 npm 包 + 平台二进制（darwin-arm64 / win32-x64）分发，无 Terrain 时 `bunx`/`npx` 降级。
- 脚本/构建：bun、`scripts/cross-windows-terrain.sh` 交叉编译。

## 系统边界

- **外部 LLM Provider**：OpenAI 兼容 HTTP；信任边界在 API key / `ModelSettings`（本地保存），`check_llm` 校验连通性。
- **ACP Agent 子进程（opencode）**：按 `AcpSettings` spawn 外部可执行文件，注入 JSON 配置与工作目录；外部代码执行边界，需 PATH 可用。
- **用户 Git 仓库**：读代码、写 `.terrain/` 知识资产；`git_policy` 约束入库内容（generated/human 冲突走"保留任一 + 重扫"）。
- **本地用户态**：`~/.terrain/registry.json`、`registry/`（sdd 会话）、`~/.terrain/bin/`（sidecar 二进制）；`path_portable` 做 `~` 路径转换。
- **Tauri IPC**：权限由 `src-tauri/capabilities/default.json` 声明；命令面见 `src/lib/api.ts`。
- **第三方依赖**：repomix-core、ts-rs、clap、tauri、tokio、time、mermaid、highlight.js 等。
- **npm 分发面**：`npm/packages/*`（cli、rtk 与平台二进制包）通过 `prepare-binaries`/`write-shims` 生成 shim。

## 代码映射索引

| 概念 | 位置 | 备注 |
|------|------|------|
| IPC 类型定义（ts-export 注解） | `crates/terrain-core/src/schema/`、`crates/terrain-core/src/ipc/` | Rust 单真源 |
| TS bindings 生成器 | `crates/terrain-ts-export/src/main.rs` | 产出 `src/lib/generated/` |
| 前端 IPC 封装 | `src/lib/api.ts`、`src/lib/types.ts`、`types.client.ts` | 只读 generated |
| Tauri 命令分发 | `src-tauri/src/commands/`、`src-tauri/src/lib.rs` | 含 assets/env/knowledge/project/sessions/workflows/usage |
| 项目注册/路径 | `crates/terrain-core/src/registry.rs`、`paths.rs`、`project.rs` | |
| 项目扫描/导入 | `crates/terrain-core/src/ingest/` | git、openapi、repo_walk |
| repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | 生成 `agent/repomix.md` |
| agent context 生成 | `crates/terrain-core/src/assets/agent_context.rs` | 生成 `agent/context.md` |
| 知识查询/搜索 | `crates/terrain-core/src/assets/query.rs`、`search.rs`、`context_layers.rs` | Macro/Meso/Micro 三层 |
| Litho C4 文档 | `crates/terrain-core/src/assets/litho.rs`、`crates/terrain-agent/src/litho.rs` | 依赖 `preset_skills/litho-documents-skill/` |
| SDD 工作流 | `crates/terrain-agent/src/workflows/sdd.rs`、`crates/terrain-core/src/assets/sdd.rs` | 四阶段 |
| ChatEngine（native/acp） | `crates/terrain-agent/src/chat/` | `native.rs`、`acp.rs`、`prompt.rs`、`tracker.rs` |
| ACP 协议客户端 | `crates/agent-client-protocol-tokio-patched/src/` | tokio patched |
| 保鲜评分 | `crates/terrain-core/src/freshness/` | ledger/git/codegraph drift |
| 环境集成 | `crates/terrain-core/src/assets/env/`、`integrations/`、`agent_tools_deploy.rs` | 依据 `env-catalog/catalog.json` |
| CLI 命令 | `crates/terrain-cli/src/commands/` | 与 IPC 共享 core |

---

已写入 `/Users/bjsttlp485/Workspace/SAW/terrain/.terrain/agent/context.md`（10,369 字符，低于 14,000 上限）。