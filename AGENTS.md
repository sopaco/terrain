# Agents Guide

This file guides AI coding agents working in this repository.

## IPC 类型（Rust ↔ TypeScript）

Tauri IPC 载荷以 **Rust 为唯一真源**；前端通过 **ts-rs** 生成 TypeScript，禁止手改生成物导致类型漂移。

### 目录与职责

| 位置 | 职责 |
|------|------|
| `crates/terrain-core/src/schema.rs`、`status.rs` 等 | 核心 IPC / 状态类型（加 `ts-export` 注解） |
| `crates/terrain-agent/src/chat/types.rs` 等 | Agent / Chat 相关 IPC 类型 |
| `crates/terrain-ts-export/` | 导出二进制，汇总需导出的根类型 |
| `src/lib/generated/` | **自动生成**，勿手改 |
| `src/lib/types.client.ts` | 纯前端类型（`ChatMessage`、`KnowledgeDoc`、`SourceSlice` 扩展等） |
| `src/lib/types.ts` | 对外入口：re-export `generated` + `types.client` |

### 修改 IPC 类型后的流程

1. 在 Rust 侧改结构体 / 枚举（保持 `Serialize`/`Deserialize` 与 ts-rs 注解一致）。
2. **新增**需导出的根类型时，在 `crates/terrain-ts-export/src/main.rs` 的 `run()` 中增加一行 `TypeName::export_all_to(&out)?`。
3. 重新生成并提交生成物：

   ```bash
   bun run gen:types
   # 等价：cargo run -p terrain-ts-export --bin export-ts-types
   ```

4. 跑 `bun run check`，按需修正前端对 `null` / 重命名的引用。

### Agent 必须遵守

- **不要**编辑 `src/lib/generated/` 下的任何文件。
- **不要**在 `types.ts` 里手写与 Rust 重复的 IPC 类型；应改 Rust 后 `gen:types`。
- UI 专用字段放在 `types.client.ts`（例如 `SourceSlice = IpcSourceSlice & { format?, focus_line? }`）。
- Rust `Option<T>` 生成结果为 **`T | null`**（不是 `undefined`）；前端判空与默认值需与此一致。
- 类型重命名用 `#[cfg_attr(feature = "ts-export", ts(rename = "..."))]`，前端通过 `types.ts` 别名保持可读性。

### ts-export 注解示例

```rust
#[cfg_attr(feature = "ts-export", derive(ts_rs::TS))]
#[cfg_attr(feature = "ts-export", ts(export, rename_all = "camelCase"))]
pub struct MyPayload { /* ... */ }
```

feature 定义见各 crate 的 `Cargo.toml`（`terrain-core`、`terrain-agent` 的 `ts-export`）。

<!-- terrain:begin env-overview v4 -->
## AI 工程环境（Terrain）

本仓库由 Terrain 配置了 AI 工程环境。Coding Agent 请遵循以下约定：

- **知识资产**位于本仓库 **`.terrain/`**（Agent 友好的知识资产、人类友好的知识库、私域知识、源码索引；可随 Git 协作）
- **项目登记**在本地 `~/.terrain/registry.json`（仅记录仓库路径，不含知识正文）
- **Skills** 位于 `.agents/skills/` 与 `.claude/skills/`（由 Terrain 注入，可按需重新集成）
- **Agent 工具**约定在 `~/.terrain/bin/`（`rtk` / `codegraph` / `terrain`）；可选本地清单 `.terrain/env/agent-tools.json`（不入库）
- **无 Terrain 安装**时：RTK / CodeGraph 可降级为 `bunx` / `npx`（见 `rtk-skill`、`codegraph-skill`）
- **工作流**：先读知识 → 再查关系 → 最后读源码；shell 输出优先走 RTK
<!-- terrain:end env-overview -->

<!-- terrain:begin knowledge-guide v5 -->
## Terrain 知识资产

Coding Agent **必须先加载** `terrain-knowledge-skill`，并按其中分层策略查询 **`.terrain/`**（仓库内路径，非全局目录）。

| 层级 | 路径 | 何时使用 |
|------|------|----------|
| Agent 友好 | `.terrain/agent/context.md` | 模块划分、核心流程、系统边界 |
| 私域 | `.terrain/knowledge/` | 业务术语、内部框架/API/脚手架 |
| 人类友好 | `.terrain/human/` | Litho 人类友好的知识库（可选参考） |
| 源码 | `.terrain/agent/repomix.md`（见 `repomix-context-skill`） | 实现细节（本地索引，不入库） |
| 关系 | codegraph CLI（见 `codegraph-skill`） | 调用链、依赖关系、影响分析 |

**原则**：先宏观后微观；优先读已生成文档，再 grep 源码索引。

## 知识资产的 Git 协作规则（必读）

`.terrain/` 的 Git 策略由 **`.terrain/.gitignore`** 与 **`.terrain/.gitattributes`** 声明（Terrain 生成并维护，随仓库分发）。

| 类别 | 位置 | Git 处理 |
|------|------|----------|
| 人为维护的私域知识 | `knowledge/` | 入库，正常三方合并 |
| 生成的知识文档 | `agent/context.md`、`human/`、`index.md` | 入库，但 **`-merge`：禁用自动合并** |
| 本机衍生物 | `agent/repomix*`、`agent/meta*.json`、`.meta/`、`env/`、`.litho-agent/`、`.sdd-agent/` | **不入库**，由 scan 本地重建 |

- **不要**把本机衍生物 `git add -f` 进版本库；它们体积大、含时间戳与 baseline git HEAD，入库必然产生冲突。
- `agent/context.md`、`human/**` 由 LLM 生成，**非确定性** —— 同一份代码两次生成措辞与结构都不同。冲突时**不要手工合并**（合并结果会是"既不是 A 也不是 B"的自相矛盾文档）：保留任一版本结束冲突，然后重新运行 Terrain scan 基于合并后的代码重生成。
- 建议知识资产的刷新集中在主干分支（或 CI）进行，feature 分支不提交 `.terrain/agent/`、`.terrain/human/` 的改动 —— 每个分支各带一份生成结果是冲突的结构性来源。`freshness` 本身就能表达"资产落后于代码"，不必每个分支都刷。

## 知识保鲜（必读）

1. 回答架构/模块问题前，优先执行 `~/.terrain/bin/terrain tools freshness --project <slug>`（或 `bunx @terrain-ai/cli tools freshness --project <slug>`）——该命令会按需重算并回写 `.terrain/.meta/freshness.json`，**不要**只静态读取该文件：它是本地缓存的快照，只在有人显式触发重算时才会更新，可能已经落后于当前 HEAD。CLI 不可用时才降级为直接读取该文件。
2. `freshness_score < 70` 时：不得仅凭 `agent/context.md` 下结论，须用 `grep repomix` 或 `codegraph` 交叉验证
3. `freshness_score < 50` 时：宏观架构上下文不可信，以 repomix 源码切片为准
4. 发现矛盾时的优先级：**repomix 源码 > codegraph > agent/context.md > human/**
5. `knowledge/` 私域文档视为人为维护；若 `refs` 指向的源码路径已删除，应降权处理
6. **CodeGraph 的 `<cg> status` 可能误报"最新"**（观察到索引 10 天未更新、期间 24 个提交改了源码，`status` 仍报正常，`query` 却查不到新符号）。做 impact/callers 分析前，先跑 `~/.terrain/bin/terrain tools codegraph-drift --project <slug>` 做独立的基于 git 的交叉验证；`likely_stale: true` 时先 `<cg> sync` 再查询（见 `codegraph-skill`）。
<!-- terrain:end knowledge-guide -->

<!-- terrain:begin skills v2 -->
### 可用 Skills

| Skill | 用途 |
|-------|------|
| `terrain-knowledge-skill` | `.terrain/` 知识分层与查询顺序（先读） |
| `repomix-context-skill` | grep/读取 `repomix.md` 源码切片 |
| `codegraph-skill` | 符号关系；`~/.terrain/bin/codegraph` 或 `bunx codegraph` |
| `rtk-skill` | 冗长 shell 加 rtk 前缀；`~/.terrain/bin/rtk` 或 `bunx @terrain-ai/rtk` |

加载顺序建议：knowledge → codegraph / repomix → rtk（执行命令时）。
<!-- terrain:end skills -->

<!-- terrain:begin tools v3 -->
### 工具链

| 工具 | 约定路径 | 无 Terrain 时降级 |
|------|----------|-------------------|
| RTK | `~/.terrain/bin/rtk` | `bunx @terrain-ai/rtk` 或 `npx @terrain-ai/rtk` |
| CodeGraph | `~/.terrain/bin/codegraph` | `bunx codegraph` 或 `npx codegraph` |
| Terrain CLI | `~/.terrain/bin/terrain` | `bunx @terrain-ai/cli` 或 `npx @terrain-ai/cli` |
| 知识文件 | `.terrain/` 仓库内路径 | 直接 Read/Grep，无需 CLI |

| 场景 | 用法 |
|------|------|
| 架构、私域知识 | 加载 `terrain-knowledge-skill` |
| 源码片段 | `repomix-context-skill`；`<rtk> grep` 搜索 pack |
| 符号关系 | `codegraph-skill`；检查 `~/.terrain/bin/codegraph` 是否存在（见 codegraph-skill） |
| git/test/build | `rtk-skill`；检查 `~/.terrain/bin/rtk` 是否存在（见 rtk-skill） |
| ACP 知识查询 | `~/.terrain/bin/terrain tools …` |
| 知识保鲜重算（自愈，勿只读静态 JSON） | `~/.terrain/bin/terrain tools freshness --project <slug>` |
| CodeGraph 独立过期检测（`<cg> status` 不可信时） | `~/.terrain/bin/terrain tools codegraph-drift --project <slug>` |

### Agent 工具解析（必读）

**一律使用约定路径**（`~/.terrain/bin/…`、`.terrain/…`），**不要**写机器相关的绝对路径（如 `/Users/…` 或 `C:\Users\…`）。

Windows 上工具部署在 `%USERPROFILE%\.terrain\bin\`（Git Bash / PowerShell 7+ 中可写为 `~/.terrain/bin/`），二进制带 `.exe` 后缀。

1. 执行前检查工具是否存在 — 见 `rtk-skill` / `codegraph-skill` 中的跨平台检查表（**不要**在 Windows 上使用 Unix 专用的 `test -x`）
2. 存在 → 用 `~/.terrain/bin/<tool> …`（词首 `~` 在 bash/zsh/Git Bash/PowerShell 7+ 会展开）
3. 不存在 → RTK / CodeGraph 用上表 `bunx` / `npx` 降级；Terrain CLI 请用户通过桌面应用操作
4. 可选参考：`.terrain/env/agent-tools.json`（本地生成、不入库），内容与约定路径一致

**不要**把 manifest 里的 `~` 路径赋给变量再引号调用（`""` 不会展开 `~`）。直接写 `~/.terrain/bin/rtk` 或选用 `bunx` 前缀。

### RTK 要点（必读 `rtk-skill`）

- **必须显式**加 rtk 前缀 — Terrain 不启用 `rtk init` 全局 hook
- 内置 Read/Grep 不会自动走 RTK — 大文件用 `<rtk> read`，搜索用 `<rtk> grep`

**注意**：不要运行 `codegraph install` 或 `rtk init`（已由 Terrain + Skills 配置）。
<!-- terrain:end tools -->