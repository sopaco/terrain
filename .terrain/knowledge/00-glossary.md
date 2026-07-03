---
type: knowledge
title: Terrain 领域术语表
---

# 领域术语表

本文件是 `.terrain/knowledge/` 私域知识层的示例内容：收录 Terrain 项目自身的业务术语、内部框架概念，供 Agent 与新成员在读 `agent/context.md` 之外，快速对齐"这个词在 Terrain 里到底指什么"。

## 知识资产相关

| 术语 | 含义 |
|------|------|
| **知识资产（Knowledge Assets）** | 存放在被扫描仓库 `.terrain/` 目录下的全部产出（`agent/`、`human/`、`knowledge/`、`.meta/`）。原位存储在仓库里，随 Git 分支演进，不是中心数据库。 |
| **Agent Pack（源码索引）** | `.terrain/agent/repomix.md` + `repomix.index.json`。由 repomix-core（`architecture-context` 策略）打包的源码切片，供 Agent `grep`/按行读取，不预加载进上下文。生成函数：`pack_agent_assets`（`crates/terrain-core/src/assets/repomix.rs`）。 |
| **Agent Context** | `.terrain/agent/context.md`。结构化架构概览（≤14 KiB），含模块地图、核心流程、系统边界。Native LLM 或 ACP 双模式生成（`crates/terrain-agent/src/agent_context.rs`）。 |
| **Litho** | 四阶段（研究→编排→输出）C4 架构文档生成流水线，产出 `.terrain/human/` 下 6 篇标准人类文档（概述/架构/工作流/深度模块/边界接口/数据库概览）。断点续传状态存于 `.terrain/.litho-agent/`。 |
| **DeepWiki** | 基于知识库的问答引擎（`ChatEngine.ask`），按 Macro/Meso/Micro 三层渐进式取材，回答带来源引用（`SourceCitation`）。 |
| **SDD（Standardized Development Workflow）** | 需求 → 技术设计 → 代码生成 → 代码评审 四阶段标准开发流程；前两阶段和评审用 Native LLM，代码生成阶段经 ACP Agent 执行。产物默认落在 `~/.terrain/sdd/{project}/sessions/{id}/outputs/`（本地、不入库）。 |
| **Freshness Ledger（新鲜度账本）** | `.terrain/.meta/freshness.json`。记录 `agent_pack`/`agent_context`/`human_docs` 三层各自的 0–100 分新鲜度评分与漂移因子（`FreshnessDriftFactor`），综合分取三层最小值。由 `compute_freshness` / `resolve_freshness_summary`（`crates/terrain-core/src/freshness.rs`）计算，是本地缓存快照，需显式重算才更新。 |
| **Codegraph Drift Report** | 独立于 CodeGraph 自身 `status` 命令的漂移检测：对比 `.codegraph/codegraph.db` 的 mtime 与 `git log --since`，用于识别"`status` 误报新鲜"的场景（见 `codegraph_drift`，`crates/terrain-core/src/freshness.rs`）。 |

## 三层知识渐进取材模型

DeepWiki/Agent 回答问题时按以下顺序取材，避免一次性把全部知识灌入上下文：

| 层级 | 路径 | 取材方式 |
|------|------|----------|
| **Macro（宏观）** | `.terrain/agent/context.md` | `read-context`，预加载整体架构概览 |
| **Meso（中观）** | `.terrain/human/`、`.terrain/knowledge/` | `search` / `read-doc`，按需取特定文档或章节 |
| **Micro（微观）** | `.terrain/agent/repomix.md` | `grep-pack` → `read-pack-file`，定位后再读具体源码切片 |

## 项目/仓库相关

| 术语 | 含义 |
|------|------|
| **Project Slug** | 仓库在 Terrain 里的唯一标识（如本项目为 `terrain`），映射关系存于本机 `~/.terrain/registry.json`（`RegistryEntry { slug, repo_path }`），不含知识正文，可随时重建。 |
| **ACP（Agent Client Protocol）** | Terrain 与外部编码 Agent（默认 `opencode`，可通过 `TERRAIN_ACP_BINARY` / `TERRAIN_ACP_COMMAND` 覆盖）之间的进程间协议。Litho 全流水线、SDD 代码生成、以及可选的 Agent 上下文生成都可以路由到 ACP Agent 执行，而非仅用 Native LLM。 |
| **Env Catalog（环境目录）** | `env-catalog/`：Skills 定义、`AGENTS.md` 片段模板（`agents-md/*.fragment`）、工具集成清单（`catalog.json`）的权威来源。目标仓库的 `.agents/skills/`、`AGENTS.md` 中的 Terrain 托管片段都是从这里注入/同步的。 |
| **Bundled Tool（内置工具）** | CodeGraph CLI、RTK CLI——Terrain 桌面应用自带二进制，部署到目标仓库时软链接到 `~/.terrain/bin/`，无需目标仓库自行安装。 |

## 命名与约定提醒

- 文中"Agent"在不同语境下可能指：(a) 桌面应用里与用户对话的 DeepWiki/Chat Agent，(b) 通过 ACP 调用的外部编码 Agent（如 OpenCode），(c) 泛指任意消费 `.terrain/` 知识的 Coding Agent。阅读时注意上下文区分，避免混淆。
- "Pack" 单独出现时默认指 Agent Pack（`repomix.md`），不要与 npm/cargo 的 "package" 概念混淆。
