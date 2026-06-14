import { TERMS } from "./terminology";

export type GlossaryEntry = {
  term: string;
  description: string;
};

/** In-app user dictionary — Chinese primary labels with short explanations. */
export const GLOSSARY: GlossaryEntry[] = [
  {
    term: TERMS.knowledgeTab,
    description:
      "浏览仓库内 `.mind-mesh/` 下的全部知识资产，包含人类友好与 Agent 友好两类文档及结构化索引。",
  },
  {
    term: TERMS.humanKnowledge,
    description:
      "由 Litho 流水线生成、存放在 `human/` 的 C4 架构文档（如 1.概述、2.架构），面向团队成员阅读与 onboarding。",
  },
  {
    term: TERMS.agentKnowledge,
    description:
      "存放在 `agent/context.md` 等路径，面向 Ask 与 Coding Agent 的密集架构说明（模块地图、核心流程等），不是长文阅读材料。",
  },
  {
    term: "添加并初始化",
    description: `选择本地 Git 仓库后，自动完成扫描索引、重建源码索引、生成 ${TERMS.agentKnowledge} 与 ${TERMS.humanKnowledge}（视 LLM / ACP 配置而定）。`,
  },
  {
    term: "重建源码索引",
    description:
      "使用 Repomix 重新打包仓库源码到 `agent/repomix.md`，供问答与 Agent 按路径检索代码（不入库，可重复生成）。",
  },
  {
    term: "问答",
    description: `基于当前项目 ${TERMS.knowledgeTab} 与源码索引的 DeepWiki 式对话，可引用文档片段与代码行号。`,
  },
  {
    term: "快速保鲜",
    description:
      "扫描 + 重建源码索引 + 重新生成 Agent 友好的知识资产（跳过 Litho）。代码变更后用于降低过期知识对 Agent 的误导。",
  },
  {
    term: "知识新鲜度",
    description:
      "对比 Git HEAD 与知识资产生成时的 baseline 提交，计算 0–100 分。低于 50 分时 Ask 不会预加载可能过期的架构概览。",
  },
  {
    term: TERMS.agentEnv,
    description: `为 Cursor 等 Coding Agent 集成 Skills、CodeGraph、RTK 与 AGENTS.md，与「${TERMS.humanKnowledge}」生成相互独立。`,
  },
  {
    term: "结构化索引",
    description:
      "从 OpenAPI 等自动导入的接口、路由、事件文档，以及开发者维护的 meta 输入。",
  },
  {
    term: "ACP",
    description: `Agent Client Protocol 代理（默认 OpenCode），用于生成 ${TERMS.humanKnowledge} 等需要外部编码 Agent 的任务。`,
  },
  {
    term: "LLM",
    description: `大语言模型连接（OpenAI 兼容 / LM Studio / Ollama），用于生成 ${TERMS.agentKnowledge} 与本地问答。`,
  },
  {
    term: "SDD 工作流",
    description:
      "软件设计驱动四阶段流程：需求澄清 → 技术设计 → 实现记录 → 代码评审，产出保存在 `.sdd-agent/`。",
  },
];
