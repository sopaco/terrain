---
type: agent_context
project: mind-mesh
title: Agent Architecture Context
source: /Users/bjsttlp485/Workspace/SAW/mind-mesh
---

## 项目概览

MindMesh 是一款工程环境管理平台，旨在为 AI 编码助手（如 OpenCode）提供自动化项目知识注入。

系统接收 Git 仓库源码，自动扫描并分析 C4 模型上下文，生成 AI 友好的结构化知识资产（`agent/context.md`）和代码映射（`agent/repomesh.md`），以及人类可读的 Litho 文档。

核心能力包括源码解析、RAG 检索增强及 C4 架构自动生成。

**依赖项**：Tauri (桌面), Rust, Node.js, Mermaid, OpenCode (协议)。