---
type: agent_context
project: terrain
title: Agent Architecture Context
source: .
---

## 项目概览
Terrain 是面向 AI 编码助手的工程环境管理平台，核心理念：**Terrain prepares the ground so agents don't have to guess where to stand**。为人类开发者与外部 Coding Agent 提供“有地图、有路标、有规范”的工程领地。核心能力：智能代码分析、C4 架构文档生成（Litho）、DeepWiki 知识问答、SDD 四阶段工作流、环境集成。知识存储在仓库内 `.terrain/` 目录，随 Git 分支流转。

## 架构设计
**容器分层**：
- **Frontend**：Svelte + Tauri 桌面应用（src-tauri/）
- **CLI**：terrain-cli（Rust），提供 ask、init、sdd、project 等命令
- **Core**：terrain-core（Rust），核心领域逻辑：扫描、打包、知识搜索、环境管理
- **Agent**：terrain-agent（Rust），Agent 运行时：ChatEngine、上下文生成、工具执行
- **TS Export**：terrain-ts-export（Rust），TypeScript 绑定生成
- **Patched Protocol**：agent-client-protocol-tokio-patched，ACP 协议实现

**依赖关系**：CLI → Core；Agent → Core；Frontend → Agent（IPC）；所有 Rust crate 共享 terrain-core 类型。

## 模块地图
| 模块 | 职责 | 主要路径 |
|------|------|----------|
| terrain-core | 核心领域：扫描、打包、知识索引、环境管理 | crates/terrain-core/src/ |
| terrain-agent | Agent 运行时：ChatEngine、上下文生成、工具执行 | crates/terrain-agent/src/ |
| terrain-cli | 命令行入口：ask、init、sdd、project、tools | crates/terrain-cli/src/commands/ |
| terrain-ts-export | TypeScript 绑定导出 | crates/terrain-ts-export/src/ |
| agent-client-protocol | ACP 子进程通信协议 | crates/agent-client-protocol-tokio-patched/src/ |
| src-tauri | 桌面应用后端：Tauri 命令、托盘、预设技能 | src-tauri/src/commands/ |
| frontend-svelte | 前端 UI：DeepWiki 面板、SDD 面板、Ask 栏 | src/lib/components/ |
| skills | 可插拔技能：codegraph、repomix、rtk、knowledge | preset_skills/、skills/ |
| env-catalog | 环境目录：Skills、CLI、tools 模板 | env-catalog/、npm/ |

## 核心流程
1. **项目初始化**：用户通过 CLI/桌面应用注册 Git 仓库 → ProjectScanner 采集元数据、OpenAPI 规范 → 生成 .terrain/ 目录结构
2. **知识生成**：repomix-core 将源码打包为 agent/repomix.md → Litho 四阶段流水线（预处理→C4 研究→编排→输出）生成 human/ 文档与 agent/context.md
3. **DeepWiki 问答**：ChatEngine 接收问题 → Macro 预载 context.md → Meso 搜索 human/knowledge → Micro grep/read repomix → 返回带引用的答案
4. **SDD 工作流**：四阶段（需求→技术设计→代码生成→代码审查）→ 轻量文档阶段走 Native LLM → 代码生成委托 ACP Agent

## 技术选型
- **语言**：Rust（核心/CLI/Agent）、TypeScript/Svelte（前端）、Node.js（工具链）
- **桌面框架**：Tauri（Rust + Web 前端）
- **LLM 集成**：Native LLM + ACP 子进程双后端
- **知识打包**：repomix-rs（grep 友好的源码索引）
- **文档生成**：Litho（C4 架构文档自动生成）
- **代码图谱**：codegraph（本地代码知识图谱）
- **分发**：npm 包（terrain、rtk）、预编译二进制（darwin-arm64、win32-x64）

## 系统边界
**外部依赖**：
- LLM 提供商（OpenAI/Anthropic 等 API）
- Git 仓库（本地/远程）
- OpenAPI 规范文件
- 文件系统（.terrain/ 知识目录）
- npm 生态（工具链安装）

**边界与信任**：
- 外部 Coding Agent 通过 ACP 协议与 terrain-agent 通信
- 桌面应用通过 Tauri IPC 与后端交互
- 知识资产（human/、agent/）随代码库版本控制，无中心化数据库
- 环境集成涉及第三方 Skills 安装，存在供应链信任边界

## 代码映射索引
| 概念 | 位置 | 备注 |
|------|------|------|
| 核心领域模型 | crates/terrain-core/src/lib.rs | 类型定义、错误处理 |
| 项目扫描 | crates/terrain-core/src/repo_walk.rs | Git 遍历、文件分类 |
| repomix 打包 | crates/terrain-core/src/repomix.rs | 源码索引生成 |
| 知识搜索 | crates/terrain-core/src/search.rs | 全文检索 |
| Agent 上下文生成 | crates/terrain-agent/src/context_generator.rs | 三层模型（Macro/Meso/Micro） |
| ChatEngine | crates/terrain-agent/src/chat/mod.rs | 对话编排、工具调用 |
| ACP 子进程 | crates/terrain-agent/src/chat/acp.rs | ACP 协议实现 |
| CLI 命令 | crates/terrain-cli/src/commands/ | ask.rs、init.rs、sdd.rs |
| 桌面后端 | src-tauri/src/commands/ | Tauri 命令映射 |
| 前端组件 | src/lib/components/ | Svelte UI 组件 |
| 预设技能 | preset_skills/ | agent-architecture、ask、litho、sdd |
| 环境目录 | env-catalog/ | Skills、tools、AGENTS.md 片段 |
| TS 绑定 | crates/terrain-ts-export/bindings/ | .d.ts 生成 |
| 知识存储 | .terrain/knowledge/ | Markdown 文档 |
| 源码包 | .terrain/agent/repomix.md | grep 友好索引 |