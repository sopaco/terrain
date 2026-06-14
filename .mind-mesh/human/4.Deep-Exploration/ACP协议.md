# ACP 协议领域

**模块路径**：`crates/mind-mesh-agent/src/acp.rs`
**生成日期**：2026-06-14
**分析置信度**：8/10

---

## 这个模块在做什么

ACP 协议模块是 MindMesh 与 OpenCode（或其他 ACP 兼容代理）之间的"外交官"。它不会自己执行复杂的 LLM 任务（如文档生成），而是通过 ACP 协议构建通信配置、管理环境变量注入、检测代理可用性，把任务委托给专门的代理执行。

---

## 核心功能点

1. **配置构建**——`build_acp_config()` 创建 `AcpAgentConfig`，支持自动批准、工作目录、环境变量注入。
2. **可用性检测**——`acp_available()` 检查二进制文件是否在 PATH 中。
3. **命令构建**——`acp_spawn_command()` 构建 ACP 启动命令，支持三种覆盖方式。

## 关键组件

| 组件/类型 | 文件路径 | 核心职责 |
|---------|---------|---------|
| `build_acp_config()` | `crates/mind-mesh-agent/src/acp.rs:61` | 构建 ACP Agent 配置 |
| `acp_available()` | `crates/mind-mesh-agent/src/acp.rs:47` | 检测 ACP 代理是否已安装 |
| `acp_spawn_command()` | `crates/mind-mesh-agent/src/acp.rs:31` | 构建启动命令字符串 |

**分析置信度**：8/10 — 完整阅读了 acp.rs 全部 88 行源码。
