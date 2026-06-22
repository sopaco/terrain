# Agent 上下文领域

**模块路径**：`crates/terrain-agent/src/agent_context.rs`
**生成日期**：2026-06-14
**分析置信度**：8/10

---

## 这个模块在做什么

Agent 上下文模块负责为 AI 编码助手生成架构级别的上下文文档（`agent/context.md`）——它不包含源码本身，而是从架构层面的抽象理解。你可以把这个模块看作"AI 的入职培训师"：它把代码仓库的架构设计、模块划分、业务流程翻译成 AI 能够理解的叙述性文档。

与 Litho 文档（面向人类）不同，Agent 上下文（面向 AI）更注重结构化、精确性和可导航性——包含模块列表、依赖关系、核心抽象和代码路径索引。

---

## 核心功能点

1. **上下文生成**——`run_agent_context_generation()` 调用 ChatEngine 执行 LLM 调用，将分析结果写入 `agent/context.md`。
2. **资产依赖管理**——自动检查 repomix pack 是否就绪（`agent_pack_ready()`），缺失时自动调用 `pack_agent_assets()`。
3. **结果持久化**——原始 LLM 回复和洁净版分别写入 debug 文件，方便调试。

## 关键组件

| 组件/类型 | 文件路径 | 核心职责 |
|---------|---------|---------|
| `AgentContextGenerationResult` | `crates/terrain-agent/src/agent_context.rs:10` | 生成结果（output_path + meta + response_excerpt） |
| `run_agent_context_generation()` | `crates/terrain-agent/src/agent_context.rs:18` | 上下文生成主流程 |
| `agent_context_exists()` | `crates/terrain-agent/src/agent_context.rs:67` | 快捷检查函数 |

**分析置信度**：8/10 — 完整阅读了 agent_context.rs 全部 69 行源码。
