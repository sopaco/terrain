/** Help-panel glossary entries (was `src/lib/glossary.ts`). */
export default {
  knowledgeTab: {
    term: "知识资产",
    description:
      "浏览仓库内 `.terrain/` 下的全部知识资产，包含人类友好与 Agent 友好两类文档及结构化索引。",
  },
  humanKnowledge: {
    term: "人类友好的知识库",
    description:
      "由 Litho 流水线生成、存放在 `human/` 的 C4 架构文档（如 1.概述、2.架构），面向团队成员阅读与 onboarding。",
  },
  agentKnowledge: {
    term: "Agent 友好的知识资产",
    description:
      "存放在 `agent/context.md` 等路径，面向 Ask 与 Coding Agent 的密集架构说明（模块地图、核心流程等），不是长文阅读材料。",
  },
  addAndInit: {
    term: "添加并初始化",
    description:
      "选择本地 Git 仓库后，自动完成扫描索引、重建源码索引、生成 Agent 友好的知识资产 与 人类友好的知识库（视 LLM / ACP 配置而定）。",
  },
  rebuildIndex: {
    term: "重建源码索引",
    description:
      "使用 Repomix 重新打包仓库源码到 `agent/repomix.md`，供问答与 Agent 按路径检索代码（不入库，可重复生成）。",
  },
  ask: {
    term: "问答",
    description:
      "基于当前项目 知识资产 与源码索引的 DeepWiki 式对话，可引用文档片段与代码行号。",
  },
  quickRefresh: {
    term: "快速保鲜",
    description:
      "扫描 + 重建源码索引 + 重新生成 Agent 友好的知识资产（跳过 Litho）。代码变更后用于降低过期知识对 Agent 的误导。",
  },
  freshness: {
    term: "知识新鲜度",
    description:
      "对比 Git HEAD 与知识资产生成时的 baseline 提交，计算 0–100 分。低于 50 分时 Ask 不会预加载可能过期的架构概览。",
  },
  agentEnv: {
    term: "Agent 友好的工程环境",
    description:
      "为 Cursor 等 Coding Agent 集成 Skills、CodeGraph、RTK 与 AGENTS.md，与「人类友好的知识库」生成相互独立。",
  },
  structuredIndex: {
    term: "结构化索引",
    description:
      "从 OpenAPI 等自动导入的接口、路由、事件文档，以及开发者维护的 meta 输入。",
  },
  acp: {
    term: "ACP",
    description:
      "Agent Client Protocol 代理（默认 OpenCode）。在 Agent 执行模式为 ACP 时，问答、Context 生成、SDD、Litho 等任务均通过外部代理 + CLI + Skill 执行。",
  },
  llm: {
    term: "LLM",
    description:
      "大语言模型连接（OpenAI 兼容 / LM Studio / Ollama）。在 Agent 执行模式为 Native 时，用于问答、Agent Context 生成与 SDD 文档阶段。",
  },
  sdd: {
    term: "SDD 工作流",
    description:
      "软件设计驱动四阶段流程：需求澄清 → 技术设计 → 实现记录 → 代码评审，产出保存在本地 `~/.terrain/sdd/`（不入 Git）。",
  },
} as const;
