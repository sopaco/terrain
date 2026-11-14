---
name: rtk-skill
description: Compress verbose shell output with an explicit rtk prefix. Load only when a command will print many lines; built-in Read/Grep never go through rtk.
version: 2.0.0
---

# RTK Skill

`<rtk>` = `~/.terrain/bin/rtk`；无 Terrain 安装时用 `bunx @terrain-ai/rtk` 或 `npx @terrain-ai/rtk`。**不运行 `rtk init`**（Terrain 不启用全局 hook）。

## 唯一规则

仅当命令会输出很多行时，显式加前缀：`<rtk> cargo test`、`<rtk> git log -n 20 --oneline`、`<rtk> git diff`、`<rtk> read <大文件>`、`<rtk> grep "<pattern>" .`。短输出直接跑原命令。

**内置 Read / Grep / Glob 不经过 RTK**，也不会被 hook 改写。搜索与读文件优先用内置工具；只有需要压缩长输出时才用 `<rtk> read` / `<rtk> grep`。实测压缩率因命令而异：`cargo test` 很高，`grep` / `read` 只有约 15–22%，不要假定一律省 60–90%。

## 失败即降级

`<rtk>` 不存在、报错或输出为空 → 直接跑原命令，**不要重试、不要为它多加载 Skill**。RTK 保留退出码；失败时输出里的 tee 日志路径可读取完整输出，不要盲目重跑冗长命令。

## 路径约定

- 一律用约定路径，不写机器相关绝对路径；Windows 在 `%USERPROFILE%\.terrain\bin\`（`.exe`）。
- 直接写 `~/.terrain/bin/rtk …`；不要把 `~` 路径赋给变量再引号调用（`""` 不展开 `~`）。
- 可选本地清单 `.terrain/env/agent-tools.json`。

## 边界

架构问题走 `terrain-knowledge-skill`，符号关系走 `codegraph-skill`，源码切片走 `repomix-context-skill`。
