# SDD 工作流领域

**模块路径**：`crates/terrain-agent/src/sdd.rs`
**生成日期**：2026-06-14
**分析置信度**：7/10

---

## 这个模块在做什么

SDD（Standardized Development Workflow）是 Terrain 的"施工管理"——它定义了四个标准化的开发阶段（需求→技术方案→代码生成→审查），每个阶段有严格的顺序依赖。SDD 是 Terrain 中唯一涉及代码修改的工作流（CodeGen 阶段通过 ACP Agent 实现）。

---

## 核心功能点

1. **阶段执行**——`run_sdd_phase()` 执行单个 SDD 阶段。前三阶段（需求/设计/审查）使用 Native LLM，CodeGen 使用 ACP Agent。
2. **顺序依赖**——执行前检查前一阶段的输出文件是否存在（`sdd.rs:35-49`），不存在时抛出明确错误。
3. **输出持久化**——每个阶段输出独立的 Markdown 文件（如 `1.requirements.md`）。

## 关键组件

| 组件/类型 | 文件路径 | 核心职责 |
|---------|---------|---------|
| `SddPhase` | `crates/terrain-core/src/schema.rs:300` | 四阶段枚举 |
| `run_sdd_phase()` | `crates/terrain-agent/src/sdd.rs:19` | 阶段执行主函数 |

**分析置信度**：7/10 — 完整阅读了 sdd.rs 全部 187 行源码。
