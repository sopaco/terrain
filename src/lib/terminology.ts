/** User-facing Chinese labels — single source of truth for MindMesh UI copy. */
export const TERMS = {
  /** Litho C4 docs under `.mind-mesh/human/` */
  humanKnowledge: "人类友好的知识库",
  /** `agent/context.md` and related agent-facing knowledge */
  agentKnowledge: "Agent 友好的知识资产",
  /** Main tab: browse all `.mind-mesh/` documents */
  knowledgeTab: "知识库",
  /** Skills, tools, AGENTS.md integration */
  agentEnv: "Agent 友好的工程环境",
} as const;

/** Compact labels for dashboard cards and chips */
export const SHORT_TERMS = {
  agentKnowledge: "Agent友好知识资产",
  humanKnowledge: "人类友好的知识库",
} as const;

export function generateLabel(term: string, busy: boolean): string {
  return busy ? "生成中…" : `生成 ${term}`;
}
