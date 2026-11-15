---
name: codegraph-skill
description: Query symbol relationships, callers and change impact with the CodeGraph CLI. Load only for call-chain or impact questions, and skip it when the tool is unhealthy.
version: 2.0.0
---

# CodeGraph Skill

`<cg>` = `~/.terrain/bin/codegraph`，无 Terrain 安装时用 `bunx codegraph` / `npx codegraph`。**不要**运行 `codegraph install`（会另配 MCP 与 agent rules）。

## 先判断能不能用

1. 做 impact / callers 前跑 `~/.terrain/bin/terrain tools codegraph-drift --project <slug>`：`<cg> status` 可能误报「最新」，这个基于 git 的独立检测才可信。
2. `likely_stale: true` → `<cg> sync` 一次再查。
3. `sync` 失败、命令非零退出、空输出、`query` 查不到刚新增的符号 → **判定 unhealthy，立即跳过 CodeGraph**，改用 Grep 活仓库。不要重试、不要反复 sync。

## 命令

| 意图 | 命令 |
|------|------|
| 定位符号 | `<cg> query <name>` |
| 谁调用 X / X 调用谁 | `<cg> callers <symbol>` / `<cg> callees <symbol>` |
| 改动影响面 | `<cg> impact <symbol>` |
| 受影响测试 | `<cg> affected <files…>` |
| 索引状态 / 刷新 | `<cg> status` / `<cg> sync` |

索引在仓库内 `.codegraph/`；缺失时 `<cg> init -i`。

## 路径约定

一律用 `~/.terrain/bin/…`，不写机器相关绝对路径。Windows 上可能是 `codegraph.exe` / `codegraph.cmd`（`%USERPROFILE%\.terrain\bin\`）；用 `Test-Path "$HOME\.terrain\bin\codegraph*"` 检查，不要用 Unix 专用的 `test -x`。

## 边界

- 架构问题走 `terrain-knowledge-skill`；源码全文走 `repomix-context-skill`。
- 不为「验证 AST 结果」再全仓 Grep 一遍；但 unhealthy 时 Grep 是唯一可信路径。
