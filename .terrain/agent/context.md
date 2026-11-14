---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览
Terrain 是**从代码生成工程知识并保持同步**的环境管理器，形态为「桌面应用（Tauri 2）+ CLI + 外部 Agent 契约」三位一体。它把「用一个仓库做事」标准化为三类可自动产出、随 Git 流动的资产：

| 资产 | 路径 | 受众 |
|------|------|------|
| 叙述性知识库 | `.terrain/human/`（Litho C4 文档 + Mermaid） | 人 |
| 结构化 Agent 上下文 | `.terrain/agent/context.md`（≤16 KiB） | Coding Agent |
| 源码索引包 | `.terrain/agent/repomix.md` | Coding Agent（按需 grep） |

**消费者**：开发者、技术负责人、采用 AI 编码助手的团队、CI/CD 流水线、ACP 集成者（Claude Code / Codex / OpenCode）。

**关键约束**：
1. **离线优先**——扫描、打包、检索、新鲜度、环境集成全部由 Rust 内核完成，不依赖 LLM；只有生成式任务进入 `terrain-agent`。
2. **失败不覆盖好资产**——增量更新结果需通过合理性校验（如保留长度比例 ≥0.6），否则拒收。
3. **知识分层消费**——宏观 `context.md` / 中观 `human/`、`knowledge/` / 微观 `repomix.md`；冲突优先级 **repomix 源码 > CodeGraph > context.md > human/**；`freshness_score < 50` 时宏观上下文不可信。

---
## 架构设计
### 容器（Cargo workspace + 前端 + 分发）

| 容器 | 角色 | 依赖特征 |
|------|------|----------|
| `src-tauri/` | 桌面 Shell，`#[tauri::command]` IPC 网关（assets / env / knowledge / project / sessions / workflows / usage） | 依赖 `terrain-core` + `terrain-agent` |
| `src/`（Svelte 5） | 桌面 UI：项目总览、DeepWiki 问答、Litho Book 阅读器、环境/工作流面板 | 通过 IPC + `src/lib/types.ts` |
| `crates/terrain-core` | **离线内核**：扫描、打包、检索、新鲜度、环境、路径、注册表、IPC 类型 | 无 LLM 依赖 |
| `crates/terrain-agent` | **编排层**：DeepWiki Ask、上下文生成、Litho、SDD；原生 LLM 或 ACP 子进程 | `adk-agent` / `adk-acp` |
| `crates/terrain-cli` | CLI（15 个子命令）+ `terrain tools` JSON API（ACP 模式入口） | 同时调用 core 与 agent |
| `crates/terrain-ts-export` | 汇总导出 Rust IPC 类型为 TS（`bun run gen:types`） | ts-rs |
| `preset_skills/`、`env-catalog/` | Agent 契约：Skills 剧本、`AGENTS.md` 片段、工具清单模板 | 数据资源 |
| `npm/` | 分发层：`@terrain-ai/cli`、`rtk`、平台二进制包 | Node 打包脚本 |

### 分层与调用方向

```
通道层   Desktop(IPC) · CLI(clap) · terrain tools(JSON stdout) · ACP 子进程
   ↓
编排层   terrain-agent：chat(native|acp) · workflows(init/quick_refresh/ask/sdd) · tools(Agent 工具集)
   ↓
内核层   terrain-core：ingest · assets · freshness · search · env · paths/registry/schema
   ↓
存储层   仓库内 .terrain/（随分

…
## 模块地图
| 模块 | 职责 | 主要路径 |
|------|------|----------|
| 项目注册与路径 | 项目登记、slug、`.terrain/` 目录契约、跨平台路径 | `crates/terrain-core/src/registry.rs`、`paths.rs`、`project.rs`、`path_portable.rs` |
| 扫描与摄取（Scan） | 仓库遍历 → `index.md`、`modules/`、`interfaces/`、`routes/`、`events/`；OpenAPI 导入 | `crates/terrain-core/src/ingest/`（`mod.rs`、`git.rs`、`openapi.rs`）、`repo_walk.rs` |
| 源码打包（Pack） | repomix-rs 打包 → `agent/repomix.md` + `meta.json`，包新鲜度判定 | `crates/terrain-core/src/assets/repomix.rs`、`pack_read.rs` |
| Agent 上下文生成 | 提示词构建、分层切分、16 KiB 截断、baseline HEAD 记录 | `crates/terrain-core/src/assets/agent_context.rs`、`context_layers.rs`、`project_meta.rs` |
| 人类文档（Litho） | 研究/组合提示词、增量更新计划与拒绝策略、文档元数据 | `crates/terrain-core/src/assets/litho.rs`、`incremental.rs`；`crates/terrain-agent/src/litho_native.rs` |
| 新鲜度 | git 驱动评分、漂移因子、缓存账本、CodeGraph 漂移检测 | `crates/terrain-core/src/freshness/`（`compute.rs`、`scoring.rs`、`drift_factors.rs`、`ledger.rs`、`codegraph.rs`、`git.rs`） |
| 检索与引用 | 全文检索、pack grep、源码切片读取、引用组装 | `crates/terrain-core/src/search.rs`、`citations.rs`、`assets/query.rs`、`assets/pack_read.rs`、`source.rs` |
| 环境集成（Env） | catalog → plan → probe → apply；Skills/工具/`AGENTS.md` 注入 |

…
## 核心流程
### 1. 项目初始化流水线（init / refresh）

1. 注册仓库：写入 `~/.terrain/registry.json`（slug、名称、备注），计算 `.terrain/` 路径契约。
2. **离线扫描**：`ProjectScanner` 遍历 Git 仓库，产出 `index.md`、`modules/`、`interfaces/`、`routes/`、`events/`；发现 `openapi.yaml|json` 时导入接口/路由元数据。
3. **离线打包**：repomix 生成 `agent/repomix.md` 与包元数据（本机衍生物，不入库）。
4. **上下文生成**：收集 `terrain-meta.json` 声明的开发者输入（file/glob/inline）→ 构建提示词 → LLM 输出 → 强制 ≤16 KiB → 写入 `context.md` + `context-meta.json`（记录 baseline Git HEAD）。
5. **Litho 文档**：ACP 后端按研究→组合阶段产出 `human/` 六份 C4 文档，检查点留在 `.litho-agent/`；`refresh` 跳过 Litho，只做 scan + repack + 可选上下文。
6. 计算新鲜度并写 `.meta/freshness.json`；UI 刷新项目总览。

### 2. 增量更新（Git diff 驱动）

1. 读取文档 baseline HEAD 与当前 HEAD，`git diff --stat` 收集变更（文件数上限 80、diff 摘要上限 4000 字符）。
2. 判定为「文档 + 补丁」模式：模型被要求原样保留未受影响章节。
3. **合理性校验**：结果需满足最小保留长度比例等约束，否则 `reject_incremental_document` 拒收，回退全量生成——绝不覆盖既有好资产。
4. 成功后刷新 baseline HEAD，避免「永远落后于 HEAD」的无效重算。

### 3. DeepWiki Ask 三层检索

1. **宏观**：注入 `context.md` 概览 + 章节索引（上限 4500 字符）。
2. **中观**：`search_knowledge` / `read_doc` 检索 `human/` 与 `knowledge/`，单次章节返回 ≤3500 字符。
3. **微观**：`grep_agent_pack` → `read_agent_pack_file` 取源码切片（默认 150 行），必要时 `read_agent_pack_meta` 取包元数据。
4. 组装

…
## 技术选型
- **语言/构建**：Rust（workspace，`rust-toolchain.toml` 固定工具链）+ Cargo；前端 TypeScript。
- **桌面**：Tauri 2（`tauri.conf.json`、`capabilities/default.json`、托盘 `tray.rs`）。
- **前端**：Svelte 5 + Vite + bun；`svelte-check`；Markdown/Mermaid 渲染（`markdownSanitize.ts`、`mermaidLoader.ts`）。
- **Agent 运行时**：`adk-agent`、`adk-acp`、`agent-client-protocol`（ACP 子进程桥接）。
- **源码打包**：repomix-rs（crate 内 `repomix` feature），替代 Node 版 repomix。
- **类型桥**：ts-rs（`ts-export` feature）+ `terrain-ts-export` 导出二进制。
- **CLI**：clap 子命令树；JSON stdout 契约供 ACP Agent 消费。
- **数据格式**：serde / serde_json / YAML（OpenAPI）、`regex`、`tempfile`、`sys-locale`。
- **外部协作工具**：CodeGraph（`bunx codegraph`，本地 `.codegraph/codegraph.db`）、RTK（shell 输出压缩）。
- **国际化**：`src/lib/i18n/locales/{en,zh-CN}`，`scripts/check-i18n-parity.ts` 保证键对齐。
- **分发**：npm 平台包（`npm/packages/cli*`、`rtk*`）+ `scripts/write-shims.mjs` 生成 shim。

---
## 系统边界
### 外部依赖与集成点

| 边界 | 交互方式 | 备注 |
|------|----------|------|
| 本地 Git 仓库 | `git` CLI（log/diff/HEAD） | 扫描、增量、新鲜度的唯一事实来源 |
| 全局目录 `~/.terrain/` | 文件读写 | `registry.json`、`settings.json`、`bin/`、`sdd/{project}/sessions/`、env catalog |
| 仓库内 `.terrain/` | 文件读写 + `.gitignore`/`.gitattributes` | 生成资产入库（`-merge` 禁自动合并），本机衍生物不入库 |
| LLM 端点 | HTTP（模型配置） | 上下文生成、SDD 需求/设计/复审、Ask |
| ACP 子进程 | JSON-RPC over stdio | Claude Code / Codex / OpenCode，执行 Litho 组合与 SDD 代码生成 |
| 开发者元数据 | `terrain-meta.json`（root / `.terrain/` / 嵌套） | file / glob / inline 三类输入，单条默认 ≤3500 字符，prompt 汇总 ≤10000 字符 |
| CodeGraph | `bunx codegraph` / `~/.terrain/bin/codegraph` | 符号关系；`status` 可能误报，需 `codegraph-drift` 交叉验证 |
| RTK | `~/.terrain/bin/rtk` / `bunx @terrain-ai/rtk` | shell 输出压缩，降低 token 成本 |
| 文件系统副作用 | `open_path`、`process`、`shell_path` | 打开/探测外部程序 |

### 信任边界与不变量

- **优先级契约**：来源冲突时 `repomix 源码 > CodeGraph > context.md > human/`；`freshness_score < 50` 降权宏观层，`< 70` 需交叉验证。
- **非确定性资产**：`context.md` 与 `human/**` 由 LLM 生成，冲突时**禁止手工合并**，应结束冲突后重新 scan 生成。
- **防御性写入**：增量结果须通过结构校验才落盘；Litho 采用研究检查点 + 可恢复模式。
- **离线可测**：内核不依赖网络，环境探测与新鲜度计算带缓存与显式失效机制。

---
## 代码映射索引
| 概念 | 位置 | 说明 |
|------|------|------|
| 路径契约与项目注册 | `crates/terrain-core/src/paths.rs`、`registry.rs` | `KnowledgePaths` 为全部资产路径权威 |
| 仓库扫描器 | `crates/terrain-core/src/ingest/mod.rs` | `ProjectScanner::scan_repo` |
| OpenAPI 导入 | `crates/terrain-core/src/ingest/openapi.rs` | `OpenApiImporter`、spec 发现与去重 |
| 源码打包 | `crates/terrain-core/src/assets/repomix.rs` | `pack_agent_assets`、`agent_pack_fresh` |
| 上下文生成与层切分 | `crates/terrain-core/src/assets/agent_context.rs`、`context_layers.rs` | 16 KiB 上限、概览 4500、章节 3500 |
| 开发者元数据收集 | `crates/terrain-core/src/assets/project_meta.rs` | `collect_project_meta`、`format_meta_bundle_for_prompt` |
| 增量更新校验 | `crates/terrain-core/src/assets/incremental.rs` | `plan_incremental_update`、`reject_incremental_document` |
| 新鲜度计算 | `crates/terrain-core/src/freshness/compute.rs`、`scoring.rs` | `compute_freshness` → `.meta/freshness.json` |
| 环境集成 | `crates/terrain-core/src/assets/env/`（`catalog.rs`、`plan.rs`、`status/`、`apply.rs`、`agents_md.rs`） | plan/probe/apply 三阶段 |
| Agent 工具集 | `crates/terrain-agent/src/tools.rs` | `read_agent_context_tool`、`grep_agent_pack_tool`、`read_agent_pack_file_tool` 等 |
| LLM/ACP 后端 | `cr

…
## 项目概览
Terrain 是**从代码生成工程知识并保持同步**的环境管理器，形态为「桌面应用（Tauri 2）+ CLI + 外部 Agent 契约」三位一体。它把「用一个仓库做事」标准化为三类可自动产出、随 Git 流动的资产：

| 资产 | 路径 | 受众 |
|------|------|------|
| 叙述性知识库 | `.terrain/human/`（Litho C4 文档 + Mermaid） | 人 |
| 结构化 Agent 上下文 | `.terrain/agent/context.md`（≤16 KiB） | Coding Agent |
| 源码索引包 | `.terrain/agent/repomix.md` | Coding Agent（按需 grep） |

**消费者**：开发者、技术负责人、采用 AI 编码助手的团队、CI/CD 流水线、ACP 集成者（Claude Code / Codex / OpenCode）。

**关键约束**：
1. **离线优先**——扫描、打包、检索、新鲜度、环境集成全部由 Rust 内核完成，不依赖 LLM；只有生成式任务进入 `terrain-agent`。
2. **失败不覆盖好资产**——增量更新结果需通过合理性校验（如保留长度比例 ≥0.6），否则拒收。
3. **知识分层消费**——宏观 `context.md` / 中观 `human/`、`knowledge/` / 微观 `repomix.md`；冲突优先级 **repomix 源码 > CodeGraph > context.md > human/**；`freshness_score < 50` 时宏观上下文不可信。

---
## 架构设计
### 容器（Cargo workspace + 前端 + 分发）

| 容器 | 角色 | 依赖特征 |
|------|------|----------|
| `src-tauri/` | 桌面 Shell，`#[tauri::command]` IPC 网关（assets / env / knowledge / project / sessions / workflows / usage） | 依赖 `terrain-core` + `terrain-agent` |
| `src/`（Svelte 5） | 桌面 UI：项目总览、DeepWiki 问答、Litho Book 阅读器、环境/工作流面板 | 通过 IPC + `src/lib/types.ts` |
| `crates/terrain-core` | **离线内核**：扫描、打包、检索、新鲜度、环境、路径、注册表、IPC 类型 | 无 LLM 依赖 |
| `crates/terrain-agent` | **编排层**：DeepWiki Ask、上下文生成、Litho、SDD；原生 LLM 或 ACP 子进程 | `adk-agent` / `adk-acp` |
| `crates/terrain-cli` | CLI（15 个子命令）+ `terrain tools` JSON API（ACP 模式入口） | 同时调用 core 与 agent |
| `crates/terrain-ts-export` | 汇总导出 Rust IPC 类型为 TS（`bun run gen:types`） | ts-rs |
| `preset_skills/`、`env-catalog/` | Agent 契约：Skills 剧本、`AGENTS.md` 片段、工具清单模板 | 数据资源 |
| `npm/` | 分发层：`@terrain-ai/cli`、`rtk`、平台二进制包 | Node 打包脚本 |

### 分层与调用方向

```
通道层   Desktop(IPC) · CLI(clap) · terrain tools(JSON stdout) · ACP 子进程
   ↓
编排层   terrain-agent：chat(native|acp) · workflows(init/quick_refresh/ask/sdd) · tools(Agent 工具集)
   ↓
内核层   terrain-core：ingest · assets · freshness · search · env · paths/registry/schema
   ↓
存储层   仓库内 .terrain/（随分

…
## 模块地图
| 模块 | 职责 | 主要路径 |
|------|------|----------|
| 项目注册与路径 | 项目登记、slug、`.terrain/` 目录契约、跨平台路径 | `crates/terrain-core/src/registry.rs`、`paths.rs`、`project.rs`、`path_portable.rs` |
| 扫描与摄取（Scan） | 仓库遍历 → `index.md`、`modules/`、`interfaces/`、`routes/`、`events/`；OpenAPI 导入 | `crates/terrain-core/src/ingest/`（`mod.rs`、`git.rs`、`openapi.rs`）、`repo_walk.rs` |
| 源码打包（Pack） | repomix-rs 打包 → `agent/repomix.md` + `meta.json`，包新鲜度判定 | `crates/terrain-core/src/assets/repomix.rs`、`pack_read.rs` |
| Agent 上下文生成 | 提示词构建、分层切分、16 KiB 截断、baseline HEAD 记录 | `crates/terrain-core/src/assets/agent_context.rs`、`context_layers.rs`、`project_meta.rs` |
| 人类文档（Litho） | 研究/组合提示词、增量更新计划与拒绝策略、文档元数据 | `crates/terrain-core/src/assets/litho.rs`、`incremental.rs`；`crates/terrain-agent/src/litho_native.rs` |
| 新鲜度 | git 驱动评分、漂移因子、缓存账本、CodeGraph 漂移检测 | `crates/terrain-core/src/freshness/`（`compute.rs`、`scoring.rs`、`drift_factors.rs`、`ledger.rs`、`codegraph.rs`、`git.rs`） |
| 检索与引用 | 全文检索、pack grep、源码切片读取、引用组装 | `crates/terrain-core/src/search.rs`、`citations.rs`、`assets/query.rs`、`assets/pack_read.rs`、`source.rs` |
| 环境集成（Env） | catalog → plan → probe → apply；Skills/工具/`AGENTS.md` 注入 |

…
## 核心流程
### 1. 项目初始化流水线（init / refresh）

1. 注册仓库：写入 `~/.terrain/registry.json`（slug、名称、备注），计算 `.terrain/` 路径契约。
2. **离线扫描**：`ProjectScanner` 遍历 Git 仓库，产出 `index.md`、`modules/`、`interfaces/`、`routes/`、`events/`；发现 `openapi.yaml|json` 时导入接口/路由元数据。
3. **离线打包**：repomix 生成 `agent/repomix.md` 与包元数据（本机衍生物，不入库）。
4. **上下文生成**：收集 `terrain-meta.json` 声明的开发者输入（file/glob/inline）→ 构建提示词 → LLM 输出 → 强制 ≤16 KiB → 写入 `context.md` + `context-meta.json`（记录 baseline Git HEAD）。
5. **Litho 文档**：ACP 后端按研究→组合阶段产出 `human/` 六份 C4 文档，检查点留在 `.litho-agent/`；`refresh` 跳过 Litho，只做 scan + repack + 可选上下文。
6. 计算新鲜度并写 `.meta/freshness.json`；UI 刷新项目总览。

### 2. 增量更新（Git diff 驱动）

1. 读取文档 baseline HEAD 与当前 HEAD，`git diff --stat` 收集变更（文件数上限 80、diff 摘要上限 4000 字符）。
2. 判定为「文档 + 补丁」模式：模型被要求原样保留未受影响章节。
3. **合理性校验**：结果需满足最小保留长度比例等约束，否则 `reject_incremental_document` 拒收，回退全量生成——绝不覆盖既有好资产。
4. 成功后刷新 baseline HEAD，避免「永远落后于 HEAD」的无效重算。

### 3. DeepWiki Ask 三层检索

1. **宏观**：注入 `context.md` 概览 + 章节索引（上限 4500 字符）。
2. **中观**：`search_knowledge` / `read_doc` 检索 `human/` 与 `knowledge/`，单次章节返回 ≤3500 字符。
3. **微观**：`grep_agent_pack` → `read_agent_pack_file` 取源码切片（默认 150 行），必要时 `read_agent_pack_meta` 取包元数据。
4. 组装

…
## 技术选型
- **语言/构建**：Rust（workspace，`rust-toolchain.toml` 固定工具链）+ Cargo；前端 TypeScript。
- **桌面**：Tauri 2（`tauri.conf.json`、`capabilities/default.json`、托盘 `tray.rs`）。
- **前端**：Svelte 5 + Vite + bun；`svelte-check`；Markdown/Mermaid 渲染（`markdownSanitize.ts`、`mermaidLoader.ts`）。
- **Agent 运行时**：`adk-agent`、`adk-acp`、`agent-client-protocol`（ACP 子进程桥接）。
- **源码打包**：repomix-rs（crate 内 `repomix` feature），替代 Node 版 repomix。
- **类型桥**：ts-rs（`ts-export` feature）+ `terrain-ts-export` 导出二进制。
- **CLI**：clap 子命令树；JSON stdout 契约供 ACP Agent 消费。
- **数据格式**：serde / serde_json / YAML（OpenAPI）、`regex`、`tempfile`、`sys-locale`。
- **外部协作工具**：CodeGraph（`bunx codegraph`，本地 `.codegraph/codegraph.db`）、RTK（shell 输出压缩）。
- **国际化**：`src/lib/i18n/locales/{en,zh-CN}`，`scripts/check-i18n-parity.ts` 保证键对齐。
- **分发**：npm 平台包（`npm/packages/cli*`、`rtk*`）+ `scripts/write-shims.mjs` 生成 shim。

---
## 系统边界
### 外部依赖与集成点

| 边界 | 交互方式 | 备注 |
|------|----------|------|
| 本地 Git 仓库 | `git` CLI（log/diff/HEAD） | 扫描、增量、新鲜度的唯一事实来源 |
| 全局目录 `~/.terrain/` | 文件读写 | `registry.json`、`settings.json`、`bin/`、`sdd/{project}/sessions/`、env catalog |
| 仓库内 `.terrain/` | 文件读写 + `.gitignore`/`.gitattributes` | 生成资产入库（`-merge` 禁自动合并），本机衍生物不入库 |
| LLM 端点 | HTTP（模型配置） | 上下文生成、SDD 需求/设计/复审、Ask |
| ACP 子进程 | JSON-RPC over stdio | Claude Code / Codex / OpenCode，执行 Litho 组合与 SDD 代码生成 |
| 开发者元数据 | `terrain-meta.json`（root / `.terrain/` / 嵌套） | file / glob / inline 三类输入，单条默认 ≤3500 字符，prompt 汇总 ≤10000 字符 |
| CodeGraph | `bunx codegraph` / `~/.terrain/bin/codegraph` | 符号关系；`status` 可能误报，需 `codegraph-drift` 交叉验证 |
| RTK | `~/.terrain/bin/rtk` / `bunx @terrain-ai/rtk` | shell 输出压缩，降低 token 成本 |
| 文件系统副作用 | `open_path`、`process`、`shell_path` | 打开/探测外部程序 |

### 信任边界与不变量

- **优先级契约**：来源冲突时 `repomix 源码 > CodeGraph > context.md > human/`；`freshness_score < 50` 降权宏观层，`< 70` 需交叉验证。
- **非确定性资产**：`context.md` 与 `human/**` 由 LLM 生成，冲突时**禁止手工合并**，应结束冲突后重新 scan 生成。
- **防御性写入**：增量结果须通过结构校验才落盘；Litho 采用研究检查点 + 可恢复模式。
- **离线可测**：内核不依赖网络，环境探测与新鲜度计算带缓存与显式失效机制。

---
## 代码映射索引
| 概念 | 位置 | 说明 |
|------|------|------|
| 路径契约与项目注册 | `crates/terrain-core/src/paths.rs`、`registry.rs` | `KnowledgePaths` 为全部资产路径权威 |
| 仓库扫描器 | `crates/terrain-core/src/ingest/mod.rs` | `ProjectScanner::scan_repo` |
| OpenAPI 导入 | `crates/terrain-core/src/ingest/openapi.rs` | `OpenApiImporter`、spec 发现与去重 |
| 源码打包 | `crates/terrain-core/src/assets/repomix.rs` | `pack_agent_assets`、`agent_pack_fresh` |
| 上下文生成与层切分 | `crates/terrain-core/src/assets/agent_context.rs`、`context_layers.rs` | 16 KiB 上限、概览 4500、章节 3500 |
| 开发者元数据收集 | `crates/terrain-core/src/assets/project_meta.rs` | `collect_project_meta`、`format_meta_bundle_for_prompt` |
| 增量更新校验 | `crates/terrain-core/src/assets/incremental.rs` | `plan_incremental_update`、`reject_incremental_document` |
| 新鲜度计算 | `crates/terrain-core/src/freshness/compute.rs`、`scoring.rs` | `compute_freshness` → `.meta/freshness.json` |
| 环境集成 | `crates/terrain-core/src/assets/env/`（`catalog.rs`、`plan.rs`、`status/`、`apply.rs`、`agents_md.rs`） | plan/probe/apply 三阶段 |
| Agent 工具集 | `crates/terrain-agent/src/tools.rs` | `read_agent_context_tool`、`grep_agent_pack_tool`、`read_agent_pack_file_tool` 等 |
| LLM/ACP 后端 | `cr

…