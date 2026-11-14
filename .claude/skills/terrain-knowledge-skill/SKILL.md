---
name: terrain-knowledge-skill
description: Use when an agent needs Terrain .terrain/ architecture or private-domain knowledge. Load only for architecture, glossary or routing questions.
version: 2.0.0
---

# Terrain Knowledge Skill

`.terrain/` 在**本仓库内**；`~/.terrain/registry.json` 只存路径映射。

## 读取顺序

1. 架构问题先跑 `~/.terrain/bin/terrain tools freshness --project <slug>`（会按需重算；勿只读静态 `.terrain/.meta/freshness.json`）。
2. 模块/流程/边界 → `.terrain/agent/context.md`（短，直接 Read）；业务术语 → `.terrain/knowledge/*.md`（按文件名排序）；结构化元数据 → `.terrain/agent/meta-inputs.md`。
3. 实现细节转 `repomix-context-skill`；符号关系转 `codegraph-skill`；冗长 shell 转 `rtk-skill`。**一次只加载需要的一份。**

## 保鲜结论怎么用

| freshness 输出 | 动作 |
|----------------|------|
| `overall_score ≥ 80` | `context.md` 可作架构依据 |
| `< 70` 或 `macro_preload_allowed: false` | 架构结论须用源码交叉验证 |
| 因子 `index_missing_new_sources` | **跳过 repomix**，直接 Grep/Read 活仓库；detail 列出缺失路径，建议重建源码索引 |
| `< 50` | 宏观上下文不可信 |

冲突优先级：**活仓库源码 > repomix 切片 > codegraph > `context.md` > `human/`**。

## 边界

- 不整包加载 `.terrain/agent/repomix.md`；按 `### <path> (<N> lines)` 标题定位切片，每次 ≤150 行。
- 不臆造与 `context.md` / `meta-inputs.md` 矛盾的模块名。
- `knowledge/` 由人维护；`refs` 指向已删除路径时降权。`human/` 体积大，非必要不读。
