import type { AskSessionInfo, ChatMessage, SourceSlice } from "../types";

export const chatSessions = $state<Record<string, ChatMessage[]>>({});
export const deepWikiSources = $state<Record<string, SourceSlice | null>>({});
export const knowledgeSources = $state<Record<string, SourceSlice | null>>({});

/** Per-project Ask conversation sessions (persisted under ~/.terrain/ask/). */
export const askSessionLists = $state<Record<string, AskSessionInfo[]>>({});
export const activeAskSessionIds = $state<Record<string, string | null>>({});

export type AskCompletionNotice = {
  projectSlug: string;
  sessionId: string;
  title: string;
  answerMarkdown: string;
  expanded: boolean;
};

export const askCompletion = $state<{ notice: AskCompletionNotice | null }>({
  notice: null,
});

export function showAskCompletionNotice(notice: Omit<AskCompletionNotice, "expanded">) {
  askCompletion.notice = { ...notice, expanded: false };
}

export function dismissAskCompletionNotice() {
  askCompletion.notice = null;
}

export function toggleAskCompletionExpanded() {
  if (!askCompletion.notice) return;
  askCompletion.notice = {
    ...askCompletion.notice,
    expanded: !askCompletion.notice.expanded,
  };
}

export function updateChat(
  slug: string,
  update: ChatMessage[] | ((prev: ChatMessage[]) => ChatMessage[]),
) {
  const prev = chatSessions[slug] ?? [];
  const next = typeof update === "function" ? update(prev) : update;
  chatSessions[slug] = next;
}

export function setDeepWikiSource(slug: string, slice: SourceSlice | null) {
  deepWikiSources[slug] = slice;
}

export function setKnowledgeSource(slug: string, slice: SourceSlice | null) {
  knowledgeSources[slug] = slice;
}

export function setAskSessions(slug: string, sessions: AskSessionInfo[]) {
  askSessionLists[slug] = sessions;
}

export function setActiveAskSessionId(slug: string, sessionId: string | null) {
  activeAskSessionIds[slug] = sessionId;
}
