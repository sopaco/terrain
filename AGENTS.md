# Agents Guide

This file guides AI coding agents working in this repository.


<!-- mind-mesh:begin env-overview v3 -->
## AI 工程环境（MindMesh）

本仓库由 MindMesh 配置了 AI 工程环境。Coding Agent 请遵循以下约定：

- **知识资产**位于本仓库 **`.mind-mesh/`**（Agent 友好的知识资产、人类友好的知识库、私域知识、源码索引；可随 Git 协作）
- **项目登记**在本地 `~/.mind-mesh/registry.json`（仅记录仓库路径，不含知识正文）
- **Skills** 位于 `.agents/skills/`（由 MindMesh 注入，可按需重新集成）
- **工作流**：先读知识 → 再查关系 → 最后读源码；shell 输出优先走 RTK
<!-- mind-mesh:end env-overview -->

<!-- mind-mesh:begin knowledge-guide v3 -->
## MindMesh 知识资产

Coding Agent **必须先加载** `mind-mesh-knowledge-skill`，并按其中分层策略查询 **`.mind-mesh/`**（仓库内路径，非全局目录）。

| 层级 | 路径 | 何时使用 |
|------|------|----------|
| Agent 友好 | `.mind-mesh/agent/context.md` | 模块划分、核心流程、系统边界 |
| 私域 | `.mind-mesh/knowledge/` | 业务术语、内部框架/API/脚手架 |
| 人类友好 | `.mind-mesh/human/` | Litho 人类友好的知识库（可选参考） |
| 源码 | `.mind-mesh/agent/repomix.md`（见 `repomix-context-skill`） | 实现细节（本地索引，不入库） |
| 关系 | codegraph CLI（见 `codegraph-skill`） | 调用链、依赖关系、影响分析 |

**原则**：先宏观后微观；优先读已生成文档，再 grep 源码索引。
<!-- mind-mesh:end knowledge-guide -->

<!-- mind-mesh:begin skills v2 -->
### 可用 Skills

| Skill | 用途 |
|-------|------|
| `mind-mesh-knowledge-skill` | `.mind-mesh/` 知识分层与查询顺序（先读） |
| `repomix-context-skill` | grep/读取 `repomix.md` 源码切片 |
| `codegraph-skill` | `bunx codegraph query/callers/callees/impact` |
| `rtk-skill` | **所有冗长 shell 命令加 `rtk` 前缀**（git/test/build/lint） |

加载顺序建议：knowledge → codegraph / repomix → rtk（执行命令时）。
<!-- mind-mesh:end skills -->

<!-- mind-mesh:begin tools v2 -->
### 工具链

| 工具 | 用法 | 场景 |
|------|------|------|
| MindMesh 知识 | 加载 `mind-mesh-knowledge-skill` | 架构、私域知识 |
| Repomix | 见 `repomix-context-skill`；`rtk grep` 搜索 pack | 源码片段 |
| CodeGraph | `bunx codegraph query/callers/callees/impact` | 符号关系、影响分析 |
| RTK | **`rtk <cmd>`** 前缀所有冗长 shell 命令 | git、test、build、lint、docker |

### RTK 要点（必读 `rtk-skill`）

- MindMesh **不启用** `rtk init` 全局 hook — Agent **必须显式**写 `rtk git status`、`rtk cargo test` 等
- 内置 Read/Grep 工具不会自动走 RTK — 大文件用 `rtk read`，搜索用 `rtk grep`
- 验证：`rtk gain`

**注意**：不要运行 `codegraph install` 或 `rtk init`（已由 MindMesh + Skills 配置）。
<!-- mind-mesh:end tools -->

## IPC 类型（Rust ↔ TypeScript）

Tauri IPC 载荷以 **Rust 为唯一真源**；前端通过 **ts-rs** 生成 TypeScript，禁止手改生成物导致类型漂移。

### 目录与职责

| 位置 | 职责 |
|------|------|
| `crates/mind-mesh-core/src/schema.rs`、`status.rs` 等 | 核心 IPC / 状态类型（加 `ts-export` 注解） |
| `crates/mind-mesh-agent/src/chat/types.rs` 等 | Agent / Chat 相关 IPC 类型 |
| `crates/mind-mesh-ts-export/` | 导出二进制，汇总需导出的根类型 |
| `src/lib/generated/` | **自动生成**，勿手改 |
| `src/lib/types.client.ts` | 纯前端类型（`ChatMessage`、`KnowledgeDoc`、`SourceSlice` 扩展等） |
| `src/lib/types.ts` | 对外入口：re-export `generated` + `types.client` |

### 修改 IPC 类型后的流程

1. 在 Rust 侧改结构体 / 枚举（保持 `Serialize`/`Deserialize` 与 ts-rs 注解一致）。
2. **新增**需导出的根类型时，在 `crates/mind-mesh-ts-export/src/main.rs` 的 `run()` 中增加一行 `TypeName::export_all_to(&out)?`。
3. 重新生成并提交生成物：

   ```bash
   bun run gen:types
   # 等价：cargo run -p mind-mesh-ts-export --bin export-ts-types
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

feature 定义见各 crate 的 `Cargo.toml`（`mind-mesh-core`、`mind-mesh-agent` 的 `ts-export`）。

