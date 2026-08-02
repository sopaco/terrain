---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览

Terrain 是面向 AI 编码助手时代的**工程环境管理平台**：知识为地图、工具为路径、流程为路标。将 Git 仓库注册后，自动扫描代码结构、生成 C4 人类文档（Litho）与机器友好上下文（`agent/context.md` + repomix 源码包），并提供 DeepWiki 知识问答与 SDD 四阶段开发工作流。知识资产存放在仓库内 `.terrain/`，随 Git 分支流转；人类通过 Tauri 桌面应用或 CLI 使用，外部 Coding Agent 通过 `terrain tools`（ACP/JSON stdout）消费同一套知识层。核心约束：Rust 为 IPC 类型唯一真源（ts-rs 生成 TS，禁手改生成物）、知识保鲜评分驱动上下文可信度、`.terrain/` 生成资产禁用自动合并。

## 架构设计

分层容器（依赖单向：core ← agent ← (cli | tauri)）：

```
Human  ──►  Tauri 桌面应用(src-tauri) ─┐
           Svelte 前端(src/)           ├─► terrain-agent ─► Native LLM / ACP 子进程
ExtAgent ─► terrain CLI(terrain-cli) ──┘        │
     │                                        ▼
     └─ terrain tools (ACP/JSON) ◄── terrain-core（无 LLM：scan/pack/search/freshness/env）
                                              │
                                              ▼
                          .terrain/ · Git · ~/.terrain/registry.json
```

| 层 | 职责 | 关键点 |
|----|------|--------|
| 表现层 | Tauri 2 桌面壳 + Svelte 5 前端 | IPC commands 薄封装 core；流式事件 AskStreamEvent |
| 编排层 | `terrain-agent` LLM 编排 | ChatEngine 双后端：Native LLM（轻任务）+ ACP 子进程（重工具任务） |
| 核心域 | `terrain-core` 无 LLM 逻辑 | scan/pack/search/freshness/env/sessions/registry |
| CLI/ACP | `terrain-cli` 命令面 | 同一三/四层知识模型暴露给外部 Agent |
| 类型契约 | `terrain-ts-export` | Rust 结构体 → TS bindings（gen:types 生成） |

关键依赖：`terrain-agent` 与 `terrain-core` 通过 workspace 内部依赖通信；ADK（`adk-acp/agent/model/runner/session/tool`）提供 LLM 与 ACP 运行时；`agent-client-protocol` 由本地 `agent-client-protocol-tokio-patched` crate patch（Cargo `[patch.crates-io]`）；`repomix-core` 生成源码包；ts-rs 产出 IPC 类型。

## 模块地图

| 模块 | 职责 | 主要路径 |
|------|------|----------|
| terrain-core | 无 LLM 核心域：扫描、打包、搜索、保鲜、env、会话、设置、schema | `crates/terrain-core/src/` |
| terrain-agent | LLM 编排：ChatEngine、Litho、SDD、上下文生成、工具运行 | `crates/terrain-agent/src/` |
| terrain-cli | CLI 命令面（ask/assets/env/init/knowledge/sdd/source/tools/usage…） | `crates/terrain-cli/src/commands/` |
| src-tauri | Tauri 2 壳：IPC commands、bundled tools、tray、preset skills 部署 | `src-tauri/src/commands/` |
| terrain-ts-export | ts-rs 汇总导出根类型（gen:types） | `crates/terrain-ts-export/src/main.rs` |
| ACP 协议 patch | agent-client-protocol-tokio 本地补丁 | `crates/agent-client-protocol-tokio-patched/` |
| 前端 | Svelte 组件/状态/渲染（markdown、mermaid、highlight、source viewer） | `src/lib/` |
| env-catalog | Skills 与 AGENTS.md 片段清单、agent-tools 模板 | `env-catalog/` |
| preset_skills | 随 App 分发的内置 skills（litho/sdd/ask/arch/context） | `preset_skills/` |
| npm 发行 | cli/rtk 跨平台二进制 shim 与版本同步 | `npm/packages/` |

## 核心流程

**1. 项目注册与知识工厂**（离线，无需 LLM）
1. `register_project` 写入 `~/.terrain/registry.json`（slug ↔ repo 指针）
2. `ProjectScanner` 采集 Git 元数据、仓库遍历、可选 OpenAPI 导入 → 产出 `index.md`
3. repomix-core 打包源码 → `agent/repomix.md` + `agent/meta.json`
4. LLM 基于 pack + developer meta 生成 `agent/context.md`；写入 `.meta/freshness.json`

**2. DeepWiki 问答**（三层检索 + 引用）
1. Macro：预载 `agent/context.md` 概览/架构/模块段
2. Meso：按需搜索/读取 `human/` 与 `knowledge/` 文档
3. Micro：`grep-pack` → `read-pack-file` 精读源码切片
4. 汇总答案并附带 `SourceCitation` 与工具调用轨迹；Native LLM 或 ACP 后端执行

**3. Litho C4 文档**（四阶段、可中断恢复）
1. 预处理：解析 markdown/代码、提取路由/接口/引用
2. C4 研究：研究产物持久化 `.terrain/.litho-agent/` 检查点
3. 编排：组合为六份标准人类文档结构（含 Mermaid）
4. 输出：写入 `human/`（1.概述~6.数据库）；LLM 生成，ACP 辅助

**4. SDD 标准化开发**（四阶段产物）
1. 需求 `1.requirements.md`（Native LLM）→ 2. 技术设计 `2.tech-design.md`（Native LLM）→ 3. 代码生成 `3.implementation.md` + 仓库改动（ACP Agent）→ 4. 代码审查 `4.code-review.md`（Native LLM）；会话输出存 `~/.terrain/sdd/{project}/sessions/{id}/outputs/`

**5. 环境集成**（探测→规划→应用）
1. 探测已有 Skills/工具/AGENTS.md 片段；生成 `EnvPlan`（按依赖序 terrain-knowledge → repomix → codegraph → rtk）
2. `apply` 部署到 `.agents/`、`AGENTS.md` 与 `~/.terrain/bin/`

## 技术选型

- 后端：Rust 2024 edition（workspace，rust-version 1.94）；tokio、serde、thiserror、tracing
- LLM/ACP 运行时：ADK 1.0（acp/agent/model/runner/session/tool）+ `agent-client-protocol`（本地 patched）
- 源码打包：`repomix-core` 2.x（grep 友好索引包）
- 类型契约：ts-rs 10 + `terrain-ts-export` 二进制（`bun run gen:types`）
- 桌面壳：Tauri 2（commands、capabilities、tray、bundled resources）
- 前端：Svelte 5 + Tailwind 4 + Vite 8 + TypeScript；marked（markdown）、mermaid（图）、highlight.js（源码高亮）、html2canvas（分享图）
- 基础设施：Git（仓库内知识流转）、`~/.terrain/registry.json` 本地注册表、CodeGraph（符号关系）、RTK（shell 输出压缩）

## 系统边界

- **外部 LLM API**：OpenAI / Ollama / LM Studio（`adk-model`），经 `ModelSettings`/`AcpSettings` 配置，有默认 base URL/model
- **ACP Agent 子进程**：`DEFAULT_ACP_BINARY/ARGS` 启动外部 Agent；`agent-client-protocol-tokio-patched` 承载协议
- **文件系统**：`.terrain/`（知识，随 Git 分支）、`~/.terrain/`（registry、sdd 会话、bin 工具、bundled preset skills）
- **外部工具**：CodeGraph CLI、RTK、`terrain` CLI 本体（bundled_tools 发现/部署）
- **信任边界**：桌面 App 内集成 sidecar 工具与子进程（shell/命令执行），路径经 `path_portable` 归一化、`process` 包装；IPC 载荷以 Rust 为唯一真源，前端仅消费 ts-rs 生成类型
- **第三方集成**：OpenAPI 规范导入（只读）、git 命令只读/写状态受限

## 代码映射索引

| 概念 | 位置 | 说明 |
|------|------|------|
| 项目扫描/OpenAPI/Git 采集 | `crates/terrain-core/src/ingest/` | ProjectScanner、ScanReport |
| Repomix 打包 | `crates/terrain-core/src/assets/repomix.rs` | pack_agent_assets |
| Agent 上下文生成 | `crates/terrain-core/src/assets/agent_context.rs` | write_agent_context、meta inputs |
| Litho 流水线 | `crates/terrain-core/src/assets/litho.rs`、`crates/terrain-agent/src/litho.rs` | 四阶段、检查点 |
| DeepWiki/Ask | `crates/terrain-core/src/assets/ask.rs`、`crates/terrain-agent/src/chat/` | 三层检索、双后端 |
| SDD 工作流 | `crates/terrain-core/src/assets/sdd.rs`、`crates/terrain-agent/src/workflows/sdd.rs` | 四阶段产物 |
| 保鲜评分 | `crates/terrain-core/src/freshness/` | compute/scoring/git/codegraph/ledger |
| 环境集成 | `crates/terrain-core/src/assets/env/`、`src-tauri/src/commands/env.rs` | 探测/计划/应用 |
| 会话与设置 | `crates/terrain-core/src/sessions/`、`settings.rs` | Ask/SDD session、ModelSettings |
| IPC/schema 类型 | `crates/terrain-core/src/schema/`、`crates/terrain-agent/src/chat/types.rs` | ts-rs 注解、Rust 唯一真源 |
| CLI 命令面 | `crates/terrain-cli/src/commands/` | ask/env/init/sdd/tools/usage 等 |
| Tauri IPC 命令 | `src-tauri/src/commands/` | assets/env/knowledge/project/workflows |
| 前端 IPC 绑定 | `src/lib/api.ts`、`src/lib/types.ts`、`src/lib/generated/` | 生成物勿手改 |
| ACP 工具执行 | `crates/terrain-agent/src/tools.rs`、`acp.rs`、`runtime.rs` | 工具 schema/session 缓存 |