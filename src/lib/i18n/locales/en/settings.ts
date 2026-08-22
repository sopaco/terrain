export default {
  language: {
    label: "Language / 语言",
    system: "Follow system",
    hint: "Affects UI copy, CLI output, the language of knowledge assets, and agent replies.",
    assetsStale:
      "Language switched. Existing knowledge assets are still in the previous language — run Quick Refresh or Regenerate from the project overview to update them.",
  },
  title: "Settings",
  subtitle:
    "ACP agent, execution mode, and knowledge-asset freshness (saved to ~/.terrain/settings.json)",
  executionMode: {
    label: "Execution mode",
    pureAcp: "Pure ACP mode",
    hybridOption: "Native LLM (BYOK) + ACP",
  },
  modeNote: {
    pureAcpBefore:
      "Q&A, Litho, SDD, and agent context are all handled by the external ACP agent — just configure the ACP command below; no Native LLM needed. Default: ",
    pureAcpAfter: ".",
    hybrid:
      "Native LLM (BYOK) handles Q&A, SDD document phases, and Agent Context, with streaming output and tool-call details; ACP handles Litho and SDD code generation. Configure both sections below.",
  },
  provider: {
    openai: "OpenAI-compatible (NVIDIA Integrate, etc.)",
    lmstudio: "LM Studio (local)",
    ollama: "Ollama (local)",
  },
  acp: {
    title: "ACP Agent",
    binary: "Binary (executable on PATH)",
    args: "Arguments (after the binary)",
    commandOverride:
      "Full command override (optional, takes precedence over binary + args)",
    test: "Test ACP agent",
    testOk: "Detected",
    testFailed: "Not detected — check the binary or command",
  },
  llm: {
    title: "Native LLM",
    apiMode: "API mode",
    apiModeChatCompletions: "Chat Completions (/v1/chat/completions)",
    apiModeResponses: "Responses API (/v1/responses)",
    apiModeHint:
      "Some newer models and GitHub Copilot proxies only expose the OpenAI Responses API. Use Chat Completions for classic OpenAI-compatible endpoints.",
    test: "Test LLM connection",
    testOk: "Connected",
    testFailed: "Connection failed — check the configuration",
  },
  freshness: {
    title: "Knowledge-asset freshness",
    hint: "Applies only to Quick Refresh and automatic refresh. Regenerate on the overview page always rebuilds from scratch.",
    incremental: "Update knowledge assets incrementally",
    incrementalHint:
      "For Git repos with existing knowledge assets, the git diff since the baseline commit is sent to the model to make local revisions on existing docs; when off, every refresh regenerates everything from scratch.",
    maxChangedFiles:
      "Max changed files (falls back to full regeneration when exceeded)",
    maxChangedFilesHint:
      "When the change set is too large, the diff no longer represents the change, and incremental updates are neither faster nor more reliable. Default: {count}.",
    syncHumanDocs:
      "Also incrementally update the human-friendly knowledge base during Quick Refresh",
    syncHumanDocsHint:
      "Litho is the slowest step and is not part of Quick Refresh by default. When enabled, affected human/ docs are revised in place from the diff — still faster than regenerating, but noticeably lengthens Quick Refresh. On first enable, the current code is recorded as the baseline without checking whether existing docs are stale — if you suspect they are, run Regenerate first.",
  },
  saveHint:
    "Each provider's configuration is saved separately to ~/.terrain/settings.json. Defaults from `.env` are only read when no settings file exists.",
} as const;
