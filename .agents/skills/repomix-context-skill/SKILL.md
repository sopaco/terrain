---
name: repomix-context-skill
description: Read a source slice from the local .terrain/agent/repomix.md index. Load only for implementation detail questions; never load the whole pack.
version: 2.0.0
---

# Repomix Context Skill

本地源码索引 `.terrain/agent/repomix.md`（gitignored，由 Terrain scan 重建）。

## 先确认索引可用

freshness 含 `index_missing_new_sources` 或 `macro_preload_allowed: false` 时，**不要用本 Skill**：索引缺新增源码，直接 Grep/Read 活仓库，并建议重建索引。索引与 HEAD 一致时才继续。

## 切片格式（易错）

文件节的标题是 `### <相对路径> (<行数> lines)`，例如 `### crates/terrain-core/src/lib.rs (164 lines)`。路径后**带行数括号**，所以精确匹配 `### 路径` 会 0 命中。

```bash
# 用带前缀的模式定位行号，再按行区间读，≤150 行/次
grep -n "^### crates/terrain-core/src/lib.rs " .terrain/agent/repomix.md
```

也可用 `~/.terrain/bin/terrain tools grep-pack --project <slug> --pattern <符号>` 定位；`read-pack-file` 需传 pack 内记录的相对路径，报「找不到文件」时**不要反复换路径**，直接读活仓库源码。

## 规则

- 绝不整包 Read / `cat` `repomix.md`（可达数十万 token）。
- `.terrain/agent/meta.json` 记 pack 指标与同步时间。
- 切片是**某次提交的快照**：与活仓库冲突时以活仓库为准。
- 架构问题先走 `terrain-knowledge-skill`；符号关系走 `codegraph-skill`。
- 缺索引时不自动重建（生成成本高、Windows 子进程有约束），提示用户走 Terrain UI「重建源码索引」或 `terrain assets pack-agent`。
