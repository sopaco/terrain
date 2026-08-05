---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览
Terrain 是面向 AI 编码助手时代的工程环境管理平台（"Agent 地形系统"：知识为地图、工具为路径、流程为路标）。注册一个 Git 仓库后，自动扫描源码、打包成 grep 友好的 repomix 索引（`agent/repomix.md`）、生成 C4 人类文档（Litho → `human/`）与 Agent 结构化上下文（`agent/context.md`），并注入 Skills/CLI/AGENTS.md 片段到目标仓库。知识存放在仓库内 `.terrain/`，随 Git 分支流转而非中心化数据库。人类通过桌面 App（Tauri）或 CLI 使用；外部 Coding Agent 通过 `terrain tools`（ACP 模式，JSON stdout）消费同一知识契约。关键约束：Rust 是 IPC 类型唯一真源（ts-rs 生成 TS，禁手改生成物）；`context.md` 硬上限 ≤14KiB；轻文档任务走 Native LLM、重工具任务委托 ACP 子进程；来源冲突信任序 repomix > codegraph > context.md > human/；freshness < 50 时宏观上下文须降权。

## 架构设计
Cargo workspace（edition 2024，rust-toolchain.toml 钉版本）：`terrain-core`（无 LLM 域逻辑）、`terrain-agent`（LLM/ACP 编排）、`terrain-cli`（CLI）、`terrain-ts-export`（TS 类型导出）、`src-tauri`（桌面壳）。`agent-client-protocol-tokio-patched` 通过 `[patch.crates-io]` 替换 ACP tokio 实现。

```
桌面 App / CLI ──▶ terrain-agent（编排）──▶ terrain-core（域逻辑）──▶ .terrain/ · Git · ~/.terrain/registry
                        │  ▲                                       │
                        ▼  └── Native LLM (adk-model) / ACP 子进程  └── 无 LLM：scan/pack/search/freshness/env
  前端 (Svelte5) ── Tauri v2 IPC ──▶ src-tauri/src/commands/ ──▶ core / agent
```

- **core 无 LLM 依赖**：ingest（ProjectScanner + 可选 OpenAPI 导入）、repomix 打包、search、freshness、env catalog/apply、git_policy、registry、usage、progress、render、pack_read。
- **agent 编排智能任务**：ChatEngine 双后端（`chat/native.rs` + `chat/acp.rs`）、workflows（ask/init/quick_refresh/sdd）、Litho 生成、agent context 生成、工具 schema（`tool_schema.rs`/`compat_tool.rs`）、throttle 与 tool_session_cache、agent_assets（AssetGenerator/AssetTrack）。
- **IPC 类型契约**：Rust 结构体加 `ts(export)` → `terrain-ts-export` 汇总导出到各 crate `bindings/*.ts` → 前端 `src/lib/types.ts` re-export；流程为改 Rust → `bun run gen:types` → `bun run check`。
- **知识层**：Macro（context.md 预载）→ Meso（search human/knowledge）→ Micro（grep-pack → read-pack-file）；`context_layers.rs`/`incremental.rs` 支持分层与增量刷新。
- **资产位置**：`agent/`（context/repomix）、`human/`（Litho C4 六文档）、`knowledge/`（术语）、`.meta/freshness.json`、`.litho-agent/`（研究检查点，可断点续跑）、`.terrain/env/`（agent-tools.json 可选本地清单，不入库）。

## 模块地图
| 模块 | 职责 | 主要路径 |
|------|------|----------|
| terrain-core | 无 LLM 域逻辑：扫描/打包/检索/保鲜/环境/注册/用量/进度 | `crates/terrain-core/src/` |
| terrain-agent | LLM 与 ACP 编排：问答/文档生成/SDD/上下文/工具 schema/资产 | `crates/terrain-agent/src/` |
| terrain-cli | CLI 命令面（ask/env/knowledge/project/sdd/source/tools/usage…） | `crates/terrain-cli/src/commands/` |
| src-tauri | 桌面壳：Tauri v2 IPC commands、托盘、bundled/preset 部署 | `src-tauri/src/commands/`、`tray.rs`、`bundled_tools.rs`、`preset_skills.rs` |
| 前端 | Svelte 5 UI：面板/流式聊天/来源引用/用量监控/Ask 会话 | `src/lib/components/`、`src/lib/stores/`、`src/lib/api.ts` |
| terrain-ts-export | 汇总导出 IPC TS 类型（gen:types） | `crates/terrain-ts-export/src/main.rs` |
| agent-client-protocol-tokio-patched | ACP tokio 实现的本地补丁 crate | `crates/agent-client-protocol-tokio-patched/` |
| env-catalog | 环境目录：Skills + AGENTS.md 片段（env-overview/knowledge-guide/skills/tools）+ catalog 清单 | `env-catalog/` |
| preset_skills | LLM 工作流技能（Litho/SDD/Ask/Context 架构） | `preset_skills/` |
| 发行包 | CLI/RTK 平台二进制 shim（darwin-arm64/win32-x64） | `npm/packages/`、`packages/` |

## 核心流程
1. **注册与扫描**：`terrain init`（或桌面）登记 slug↔路径到 `~/.terrain/registry.json` → ProjectScanner（ingest git 元数据 + 可选 OpenAPI 导入）产出 `index.md` → repomix 打包 `agent/repomix.md` → `terrain env apply` 部署 Skills/工具/AGENTS.md 片段。
2. **知识资产生成**：agent context 由 LLM 依 prompt + 源码包生成 `context.md`（≤14KiB）；Litho 四阶段（预处理 → C4 研究 → 编排 → 输出）产出 `human/` 六份 C4 文档，中间产物存 `.litho-agent/` 支持中断恢复；完成后写 `.meta/freshness.json` 跟踪保鲜，支持 quick_refresh 增量刷新。
3. **DeepWiki 问答**：预载 Macro `context.md` → 按需 Meso 检索 human/knowledge → Micro `grep-pack`/`read-pack-file` 读源码；Native 或 ACP 后端作答，附来源引用与工具调用轨迹（`tracker.rs` 记录）。
4. **SDD 工作流**：需求 → 技术设计 → 代码生成（ACP Agent 改仓库）→ 代码审查；每阶段产出可审查 Markdown，会话存 `~/.terrain/sdd/{project}/sessions/{id}/outputs/`。

## 技术选型
- **后端**：Rust 2024 edition（rust-toolchain.toml 钉版）、tokio、serde/serde_json、thiserror、tracing。
- **LLM**：adk 家族（adk-model，feature ollama/openai；adk-agent/runner/session/tool/core）；Native 走 `chat/native.rs`，ACP 走 `chat/acp.rs`。
- **ACP**：`agent-client-protocol` + `agent-client-protocol-tokio-patched`（本地 patch）。
- **打包**：`repomix-core`（repomix-rs，源码 → grep 友好索引）。
- **前端**：Svelte 5（runes、`*.svelte.ts` stores）+ Vite + TypeScript + Bun；marked/mermaid/highlight.js 渲染。
- **桌面**：Tauri v2（capabilities ACL、托盘、bundled resources）。
- **类型桥**：ts-rs（serde-compat），`terrain-ts-export` 二进制一键生成 bindings。
- **Git 策略**：`.terrain/.gitignore` / `.gitattributes` 区分入库知识（human/agent/ 禁自动合并）与本机衍生物（repomix*、.litho-agent/、.meta/）。

## 系统边界
- **Git 仓库（宿主）**：知识写入 `.terrain/` 随分支流转；注册表只存指针（`~/.terrain/registry.json`），不含知识正文。
- **本机状态**：`~/.terrain/registry.json`、`~/.terrain/sdd/`、`~/.terrain/bin/`（terrain/codegraph/rtk）、`.terrain/env/agent-tools.json`（可选本地清单，不入库）。
- **LLM API**：OpenAI 兼容 / Ollama / LM Studio（桌面 Settings 配置，可选）；默认 `adk-model`，无 LLM 时 scan/pack/search 仍可用。
- **ACP Agent 子进程**：OpenCode 等外部 Agent 执行 Litho 编排、SDD 代码生成、工具调用（隔离进程）；ACP 模式下 `terrain tools` JSON stdout 是唯一接口。
- **外部工具**：repomix（打包）、CodeGraph（符号关系/impact，`bunx codegraph`）、RTK（shell 输出压缩，`@terrain-ai/rtk`）。
- **信任边界**：来源冲突按 repomix > codegraph > context.md > human/ 降级；freshness_score < 50 时宏观上下文不可信、< 70 须交叉验证（git/codegraph drift）；CodeGraph status 可能误报最新，需 codegraph-drift 独立校验。

## 代码映射索引
| 概念 | 位置 | Notes |
|------|------|-------|
| 项目扫描/导入 | `crates/terrain-core/src/ingest/`（git.rs、openapi.rs） | ProjectScanner → index.md |
| repomix 打包 | `crates/terrain-core/src/assets/repomix.rs`、`assets/pack_read.rs` | agent/repomix.md |
| Agent context 生成 | `crates/terrain-agent/src/context_generator.rs`、`agent_context.rs`；`crates/terrain-core/src/assets/{agent_context,context_layers,project_meta}.rs` | prompt 组装 + LLM |
| 增量刷新 | `crates/terrain-agent/src/workflows/quick_refresh.rs`；`crates/terrain-core/src/assets/incremental.rs` | 知识保鲜快刷 |
| Litho 文档管线 | `crates/terrain-agent/src/litho.rs`；`crates/terrain-core/src/assets/litho.rs`；`preset_skills/litho-documents-skill/` | 四阶段，可断点续跑 |
| DeepWiki 问答 | `crates/terrain-agent/src/chat/`（acp.rs、native.rs、tracker.rs）；`crates/terrain-core/src/assets/ask.rs`、`query.rs` | 三层检索 + 引用 |
| SDD 工作流 | `crates/terrain-agent/src/workflows/sdd.rs`、`sdd.rs`；`crates/terrain-core/src/assets/sdd.rs` | 四阶段产物 |
| 环境集成 | `crates/terrain-core/src/assets/env/`、`agent_tools_deploy.rs`、`bundled_tools.rs`；`env-catalog/`（agents-md/*.fragment） | Skills+工具+AGENTS.md |
| Freshness | `crates/terrain-core/src/freshness/`（compute/scoring/git/codegraph/drift_factors/ledger） | .meta/freshness.json |
| IPC 命令 | `src-tauri/src/commands/`；`crates/terrain-core/src/ipc/`（chat.rs、workflows.rs） | Tauri → core/agent |
| TS 类型导出 | `crates/terrain-ts-export/src/main.rs`；各 crate `bindings/*.ts` | 勿手改生成物 |
| CLI 命令 | `crates/terrain-cli/src/commands/`（ask/tools/env/sdd…） | 含 `terrain tools` ACP |
| LLM/ACP 运行时 | `crates/terrain-agent/src/runtime.rs`、`throttle.rs`、`tool_session_cache.rs`、`builder.rs`；`crates/agent-client-protocol-tokio-patched/` | adk + 本地 patch |
| 前端入口 | `src/lib/api.ts`、`src/lib/stores/`、`src/lib/components/`、`src/lib/{askSession,resolveSource,assistantSteps}.ts` | Svelte 5 runes |
| 路径/注册 | `crates/terrain-core/src/paths.rs`、`registry.rs`、`repo.rs`、`git_policy.rs`、`path_portable.rs` | .terrain 布局与 Git 策略 |
```