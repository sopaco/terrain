export default {
  language: {
    label: "语言 / Language",
    system: "跟随系统",
    hint: "影响界面文案、CLI 输出、知识资产语言与 Agent 回复语言。",
    assetsStale:
      "语言已切换。已有知识资产仍为原语言，可在项目概览中执行「快速保鲜」或「重新生成」以更新。",
  },
  title: "设置",
  subtitle: "ACP 代理、执行模式与知识资产保鲜（保存至 ~/.terrain/settings.json）",
  executionMode: {
    label: "执行模式",
    pureAcp: "纯ACP模式",
    hybridOption: "Native LLM（BYOK） + ACP",
  },
  modeNote: {
    pureAcpBefore:
      "问答、Litho、SDD、Agent 上下文等全部由外部 ACP 代理处理，只需配置下方 ACP 命令，无需填写 Native LLM。默认 ",
    pureAcpAfter: "。",
    hybrid:
      "Native LLM（BYOK）处理问答、SDD 文档阶段与 Agent Context，支持流式输出与工具调用详情；ACP 处理 Litho 与 SDD 代码生成。请同时配置下方两项。",
  },
  provider: {
    openai: "OpenAI 兼容 (NVIDIA Integrate 等)",
    lmstudio: "LM Studio (本地)",
    ollama: "Ollama (本地)",
  },
  acp: {
    title: "ACP 代理",
    binary: "Binary（PATH 上的可执行文件）",
    args: "参数（跟在 binary 后）",
    commandOverride: "完整命令覆盖（可选，优先于 binary + args）",
    test: "检测 ACP 代理",
    testOk: "检测通过",
    testFailed: "未检测到，请检查 binary 或 command",
  },
  llm: {
    title: "Native LLM",
    apiMode: "API 模式",
    apiModeChatCompletions: "Chat Completions（/v1/chat/completions）",
    apiModeResponses: "Responses API（/v1/responses）",
    apiModeHint:
      "部分新模型与 GitHub Copilot 代理仅提供 OpenAI Responses API；经典 OpenAI 兼容端点请选 Chat Completions。",
    test: "测试 LLM 连接",
    testOk: "连接正常",
    testFailed: "连接失败，请检查配置",
  },
  freshness: {
    title: "知识资产保鲜",
    hint: "仅对「快速保鲜」与自动保鲜生效。概览页的「重新生成」始终从零重新生成。",
    incremental: "增量更新知识资产",
    incrementalHint:
      "Git 仓库且已有知识资产时，把自基线提交以来的 git diff 交给大模型，在现有文档上做局部修订；关闭后每次保鲜都完整重新生成。",
    maxChangedFiles: "变更文件上限（超过则回退为完整重新生成）",
    maxChangedFilesHint:
      "变更范围过大时，diff 已不能代表这次改动，增量更新既不更快也不更可靠。默认 {count}。",
    syncHumanDocs: "快速保鲜时同步增量更新人类友好的知识库",
    syncHumanDocsHint:
      "Litho 是最慢的一步，默认不在快速保鲜中触发。开启后会按 diff 就地修订受影响的 human/ 文档，仍比重新生成快，但会明显延长快速保鲜耗时。首次开启时会直接以当前代码为基线记录，不会检查现有文档是否已经过时——如果怀疑现有文档已经过时，请先用「重新生成」。",
  },
  saveHint:
    "每个 Provider 的配置会分别保存到 ~/.terrain/settings.json。仅在没有设置文件时，才会读取 `.env` 中的默认值。",
} as const;
