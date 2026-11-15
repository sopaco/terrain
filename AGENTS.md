# Agents Guide

This file guides AI coding agents working in this repository.

## IPC 类型（Rust ↔ TypeScript）

Tauri IPC 载荷以 **Rust 为唯一真源**；前端通过 **ts-rs** 生成 TypeScript，禁止手改生成物导致类型漂移。

### 目录与职责

| 位置 | 职责 |
|------|------|
| `crates/terrain-core/src/schema/` | 核心 IPC / 状态类型，按领域拆分（`freshness.rs`、`project.rs` 等；加 `ts-export` 注解） |
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

## 子进程（Windows 控制台窗口）

Windows 打包版是 GUI 子系统（`windows_subsystem = "windows"`，无控制台）。此时拉起 `git`、`powershell`、`where.exe`、`bun`/`npx` 等控制台程序，若不带 `CREATE_NO_WINDOW`，每调用一次就闪一个黑色终端窗口 —— 知识生成、freshness、env 探测这类高频调用尤其明显。

### 必须遵守

- **只用** `terrain_core::process::{command, async_command}` 创建子进程，它们已在 Windows 上设好 `CREATE_NO_WINDOW`（非 Windows 为零成本）。
- 已有 `ShellCommand`/`Path` 的构造入口时（如打开文件管理器），优先复用 `open_path_in_file_manager` 这类已封装的函数，不要在命令层重写 `std::process::Command`。
- `crates/terrain-core/tests/process_convention.rs` 会扫描 `crates/` 与 `src-tauri/src`，发现 `process.rs` 之外的裸 `Command::new` 即失败；这是防止该问题回归的唯一可靠手段。

<!-- terrain:begin env-overview v5 -->
## AI 工程环境（Terrain）

- **知识资产**：仓库内 `.terrain/`；项目登记在 `~/.terrain/registry.json`（仅路径，不含知识正文）
- **Skills**：`.agents/skills/` 与 `.claude/skills/`；按下方「Skill 路由」**按需加载，最多一份**，不要预读全部
- **工具**：`~/.terrain/bin/`（`rtk` / `codegraph` / `terrain`）；可选本地清单 `.terrain/env/agent-tools.json`（不入库）
- **无 Terrain 安装**：RTK / CodeGraph 可降级为 `bunx` / `npx`；命令执行失败就直接跑原命令，不要重试
- **工作流**：架构问题先查知识层，实现细节直接查源码；仅冗长 shell 输出加 RTK 前缀
<!-- terrain:end env-overview -->

<!-- terrain:begin knowledge-guide v7 -->
## Terrain 知识资产

按「Skill 路由」按需加载；`.terrain/` 是仓库内路径，不是全局目录。

| 层级 | 路径 | 内容 |
|------|------|------|
| 架构 | `.terrain/agent/context.md` | 模块地图、核心流程、系统边界 |
| 私域 | `.terrain/knowledge/` | 业务术语、内部框架 / API |
| 人类 | `.terrain/human/` | Litho 文档（可选，体积大） |
| 源码 | `.terrain/agent/repomix.md` | 实现细节（本地索引，不入库） |

冲突优先级：**活仓库源码 > repomix 切片 > codegraph > `context.md` > `human/`**。

## 知识保鲜

架构/实现问题先跑 `~/.terrain/bin/terrain tools freshness --project <slug>`（会按需重算并回写 `.terrain/.meta/freshness.json`；勿只读静态 JSON，它是缓存快照）。`overall_score < 70` 或 `macro_preload_allowed: false` 时架构结论须用源码交叉验证；因子含 `index_missing_new_sources` 时按「Skill 路由」的短路规则跳过索引。分档细节见 `terrain-knowledge-skill`。

做 impact / callers 前跑 `~/.terrain/bin/terrain tools codegraph-drift --project <slug>`：`likely_stale: true` → `sync` 一次再查；`sync` 失败，或查询非零退出 / 空输出 → 降级 Grep，不要重试。`knowledge/` 由人维护，`refs` 指向已删除路径时降权。

## Git 协作规则

`.terrain/.gitignore` 与 `.terrain/.gitattributes` 由 Terrain 生成并随仓库分发：

- **入库**：`knowledge/`；`agent/context.md`、`human/`、`index.md`（生成文档设 `-merge`，冲突时不要手工合并，保留任一版本后重跑 scan）；`agent/context-meta.json`、`.meta/human.json`（记录 baseline，须随内容走）
- **不入库**：`agent/repomix*`、`agent/meta*.json`、`.meta/`、`env/`、`.litho-agent/`、`.sdd-agent/` —— 勿 `git add -f`，它们含时间戳与 baseline
- 建议只在主干刷新知识资产；feature 分支不提交 `.terrain/agent/`、`.terrain/human/`，落后程度由 freshness 表达
<!-- terrain:end knowledge-guide -->

<!-- terrain:begin skills v4 -->
### Skill 路由（按需，最多一份）

架构、实现细节、影响面任务先跑一次保鲜检查：
`~/.terrain/bin/terrain tools freshness --project <slug>`
其余任务（纯编辑、改类型、跑测试）不加载任何 Skill，直接读活仓库源码。

| 任务 | 只加载 | 跳过 |
|------|--------|------|
| 模块划分、流程、系统边界、业务术语 | `terrain-knowledge-skill` | repomix / codegraph / rtk |
| 实现细节、按路径查源码 | `repomix-context-skill` | knowledge / codegraph / rtk |
| 调用链、影响面 | `codegraph-skill` | knowledge / repomix |
| 冗长 shell 输出（git / test / build） | `rtk-skill` | 其余三份 |
| 拿不准 | `terrain-knowledge-skill`（单份 fallback） | 其余三份 |

**复合任务**（一题同时涉及多类）：按主问题选一份；确需第二份时先说明理由再加载，仍不得预读全部。

**失效短路**（依据已有输出，不额外跑探测命令）：

- freshness 含 `index_missing_new_sources`，或 `macro_preload_allowed: false` → 索引缺新增源码，**跳过 repomix 查询**，直接 Grep/Read 活仓库；`agent/context.md` 仅作背景。
- `terrain tools codegraph-drift --project <slug>` 报 `likely_stale: true` → `sync` 一次再查；`sync` 失败，或查询非零退出 / 空输出 → **跳过 codegraph**，改用 Grep 活仓库，不要重试。
- rtk 前缀执行失败 → 直接跑原命令，不要为它加载 `rtk-skill`。
<!-- terrain:end skills -->

<!-- terrain:begin tools v4 -->
### 工具链

| 工具 | 约定路径 | 无 Terrain 时 |
|------|----------|---------------|
| RTK | `~/.terrain/bin/rtk` | `bunx @terrain-ai/rtk` / `npx` |
| CodeGraph | `~/.terrain/bin/codegraph` | `bunx codegraph` / `npx` |
| Terrain CLI | `~/.terrain/bin/terrain` | `bunx @terrain-ai/cli` / `npx` |

- **一律用约定路径**，不写机器相关绝对路径；Windows 在 `%USERPROFILE%\.terrain\bin\`（Git Bash / PowerShell 7+ 可写 `~/.terrain/bin/`，二进制带 `.exe`），不要用 Unix 专用的 `test -x`。
- 不要把 manifest 里的 `~` 路径赋给变量再引号调用（`""` 不展开 `~`）。
- 工具不存在或执行失败 → 降级 `bunx` / `npx` 或直接跑原命令；Terrain CLI 不可用时请用户通过桌面应用操作。**不要重试，不要运行 `codegraph install` / `rtk init`。**
- 内置 Read / Grep 不走 RTK；仅冗长 shell 输出显式加 `~/.terrain/bin/rtk` 前缀。

| 常用命令 | 用途 |
|----------|------|
| `terrain tools freshness --project <slug>` | 知识保鲜重算（自愈，勿只读静态 JSON） |
| `terrain tools codegraph-drift --project <slug>` | CodeGraph 独立过期检测 |
<!-- terrain:end tools -->