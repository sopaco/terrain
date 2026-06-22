# CLI 接口领域

**模块路径**：`crates/terrain-cli/src/main.rs`
**生成日期**：2026-06-14
**分析置信度**：9/10

---

## 这个模块在做什么

CLI 接口是 Terrain 的"命令终端"——它通过 clap 库定义了一个完整的命令行接口，包含 6 个一级命令组（list/scan/search/read/tools/assets/env），约 30 个子命令。AI 编码助手和 CI/CD 流水线主要通过 CLI 与 Terrain 交互。

---

## 核心功能点

1. **项目管理**——`list` / `scan` / `search` / `read` 管理项目和文档。
2. **资产生成**——`assets` 命令组管理所有资产生成。
3. **ACP 工具集**——`tools` 命令组为 AI 编码助手提供结构化工具 API（所有输出为 JSON）。
4. **环境管理**——`env` 命令组管理 AI 工程环境集成。

## 关键组件

| 组件/类型 | 文件路径 | 核心职责 |
|---------|---------|---------|
| `Cli` (Parser) | `crates/terrain-cli/src/main.rs:16` | 主 CLI 结构 |
| `Commands` | `crates/terrain-cli/src/main.rs:28` | 一级命令枚举 |
| `ToolsCommands` | `crates/terrain-cli/src/main.rs:65` | ACP 工具集命令 |
| `AssetCommands` | `crates/terrain-cli/src/main.rs:122` | 资产生成命令 |
| `EnvCommands` | `crates/terrain-cli/src/main.rs:186` | 环境管理命令 |

**分析置信度**：9/10 — 完整阅读了 main.rs 全部 499 行源码。
