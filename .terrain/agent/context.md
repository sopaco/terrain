---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览
Terrain 是面向 AI 编码助手的工程环境管理平台，核心理念是“为 Agent 准备好有地图、有路标、有规范的工程领地”。它将 Git 仓库注册后自动扫描代码结构、生成 C4 架构文档（Litho）、维护双轨知识资产（`human/` 叙述文档 + `agent/` 压缩上下文），并通过 DeepWiki 问答与 SDD 四阶段工作流，让人类开发者与外部 Coding Agent 共享同一套知识契约。知识存放在仓库内的 `.terrain/` 目录，随 Git 分支流转。

## 架构设计
- **CLI 层**：`terrain-cli`（Rust）提供命令行入口
- **Core 层**：`terrain-core`（Rust）实现核心逻辑：知识 ingestion、 freshness 评分、repomix 打包、IPC 通信
- **Agent 层**：`terrain-agent`（Rust）管理 ACP/ Native 双后端、上下文生成、工具调用
- **Desktop 层**：`src-tauri`（Rust + Svelte）提供 Tauri 桌面应用
- **TS 绑定**：`terrain-ts-export`、各 crate bindings/ 生成 TypeScript 类型
- **Env 层**：`env-catalog` 管理 Skills、CLI 工具链与 AGENTS.md 片段分发
- **Preset Skills**：`preset_skills/` 内置 agent-architecture、ask、context、litho、sdd 等技能
- **外部依赖**：LLM 提供商（OpenAI/Anthropic 等）、Git、SQLite（codegraph.db）

## 模块地图
| 模块 | 责任 | 主要路径 |
|------|------|----------|
| terrain-cli | 用户 CLI 入口与项目/KB 管理 | crates/terrain-cli/src/cli.rs, crates/terrain-cli/src/commands/ |
| terrain-core | 核心引擎：扫描、freshness、ingest、repomix、IPC | crates/terrain-core/src/ |
| terrain-agent | Agent 运行时、ACP/Native 后端、上下文生成 | crates/terrain-agent/src/ |
| src-tauri | 桌面应用主程序与 IPC handlers | src-tauri/src/ |
| terrain-ts-export | TypeScript 绑定导出 | crates/terrain-ts-export/src/main.rs |
| env-catalog | Skills 目录、工具链与 AGENTS.md 模板 | env-catalog/ |
| preset_skills | 内置技能：ask、context、litho、sdd、architecture | preset_skills/*/SKILL.md |
| codegraph | 代码图知识图谱与 Neo4j 迁移数据 | packages/codegraph/ |
| rtk | Runtime Knowledge SDK | packages/rtk/ |
| npm/binaries | 平台二进制分发（darwin/win32） | packages/*/cli-*/bin/ |

## 核心流程
1. **项目注册与知识构建**：用户通过 CLI/桌面应用注册 Git 仓库 → `ProjectScanner` 采集 Git 元数据与 OpenAPI 规范 → `repomix-core` 打包源码生成 `agent/repomix.md` → 双轨知识资产（`human/` + `agent/`）初始化
2. **DeepWiki 问答（Ask）**：用户提问 → Macro 预载 `agent/context.md` → Meso 搜索 `human/knowledge/` 文档 → Micro grep/read `agent/repomix.md` 提取代码片段 → `ChatEngine` 组装带引用的回答（Native LLM 或 ACP 子进程）
3. **Litho C4 文档生成**：四阶段流水线（预处理→C4 研究→编排→输出）→ 自动产出六份标准人类文档 + Mermaid 图表 → 中间产物持久化在 `.terrain/.litho-agent/` 支持中断恢复
4. **SDD 标准化开发**：四阶段工作流（需求→技术设计→代码生成→代码审查）→ 轻量文档阶段走 Native LLM → 代码生成委托 ACP Agent → 阶段产物可审查

## 技术选型
- **核心语言**：Rust（稳定版，见 `rust-toolchain.toml`）
- **前端/桌面**：Svelte + Vite + Tauri
- **类型系统**：TypeScript（跨语言绑定）
- **包管理**：Cargo（Rust）、npm/bun（Node.js 工具链）
- **存储**：SQLite（`codegraph.db`）、Git 元数据、文件系统知识目录
- **LLM 接入**：OpenAI、Anthropic 等（通过 `model_text.rs` 抽象）
- **跨平台分发**：静态二进制（darwin-arm64、win32-x64）
- **知识检索**：repomix-rs（grep 友好）、codegraph（图查询）

## 系统边界
- **外部 API**：LLM 提供商 API（受限信任边界，需配置 API key）
- **版本控制**：Git 仓库（本地或远程，读元数据 + 写 `.terrain/`）
- **子进程**：ACP Agent 子进程（沙箱执行，单会话隔离）
- **文件系统**：用户工作区（`.terrain/` 目录，随 Git 分支同步）、系统临时目录
- **第三方工具**：Skills 目录中的可执行文件（受 host 策略约束）
- **桌面 OS**：Tauri 应用通过系统托盘与 IPC 与 Core 通信

## 代码映射索引
| 概念 | 位置 | 备注 |
|------|------|------|
| CLI 主入口 | crates/terrain-cli/src/cli.rs | 参数解析 |
| Ask 命令 | crates/terrain-cli/src/commands/ask.rs | DeepWiki 问答 CLI |
| SDD 命令 | crates/terrain-cli/src/commands/sdd.rs | 标准化开发 CLI |
| ACP Agent 运行时 | crates/terrain-agent/src/chat/acp.rs | 子进程管理 |
| Native LLM 运行时 | crates/terrain-agent/src/chat/native.rs | 内嵌 LLM |
| 上下文生成器 | crates/terrain-agent/src/context_generator.rs | agent/context.md 合成 |
| Litho 流水线 | crates/terrain-core/src/litho.rs | C4 文档生成 |
| Freshness 评分 | crates/terrain-core/src/freshness/ | 知识新鲜度计算 |
| Repomix 打包 | crates/terrain-core/src/repomix.rs | agent/repomix.md 生成 |
| IPC 消息处理 | crates/terrain-core/src/ipc/ | Core ↔ Agent 通信 |
| Tauri 主程序 | src-tauri/src/main.rs | 桌面应用入口 |
| Tauri 工作流 | src-tauri/src/commands/workflows.rs | SDD/Litho 触发 |
| Svelte 入口 | src/App.svelte | 前端根组件 |
| Skills 注册表 | crates/terrain-core/src/registry.rs | 内置 Skills 发现 |
| 项目元数据 | crates/terrain-core/src/project_meta.rs | terrain-meta.json 解析 |