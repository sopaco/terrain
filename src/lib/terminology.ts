/** User-facing Chinese labels — single source of truth for Terrain UI copy. */
export const TERMS = {
  /** Litho C4 docs under `.terrain/human/` */
  humanKnowledge: "人类友好的知识库",
  /** `agent/context.md` and related agent-facing knowledge */
  agentKnowledge: "Agent 友好的知识资产",
  /** Main tab: browse all `.terrain/` documents */
  knowledgeTab: "知识资产",
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

/** StatusBanner chip labels */
export const STATUS_CHIP_LABELS = {
  idle: "就绪",
  loading: "加载中",
  progress: "进行中",
  success: "完成",
  error: "错误",
} as const;

/** Common user-facing error / hint messages */
export const UI_MESSAGES = {
  selectProjectWithRepo: "请先选择已关联仓库路径的项目。",
  selectProjectWithRepoPath: "请先选择已关联仓库路径的项目。",
  openFolderFailed: (e: unknown) => `打开文件夹失败：${e}`,
  agentContextGenerating: "正在生成 Agent 友好的知识资产…",
  agentContextReady: "Agent 友好的知识资产已就绪",
  loadingDocument: "正在加载文档…",
  selectProject: "选择项目",
  noProject: "未选择项目",
  noProjectSelected: "未选择项目。",
  askPlaceholder: "向本项目提问…",
  askFollowUpPlaceholder: "继续追问…",
  loadingDocs: "加载中",
  repacking: "重新打包中…",
  repack: "重新打包",
} as const;

/** DeepWiki streaming phase labels */
export const CHAT_PHASE_LABELS = {
  preparing_pack: "正在打包源码索引…",
  preparing_context: "正在生成架构上下文…",
  thinking: "思考中…",
  tools: "调用工具中…",
  generating: "生成回答中…",
  streaming: "流式输出中…",
} as const;

/** Tool call trace labels */
export const TOOL_LABELS: Record<string, string> = {
  list_projects: "列出项目",
  search_knowledge: "搜索知识库",
  read_doc: "读取文档",
  read_agent_pack_meta: "读取打包元数据",
  grep_agent_pack: "搜索 Agent 包",
  read_agent_pack_file: "读取源码文件",
  list_human_docs: "列出人类文档",
};

export const TOOL_STATUS_LABELS = {
  running: "运行中",
  error: "失败",
  ok: "完成",
} as const;
