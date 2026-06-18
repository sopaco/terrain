import type { ChatMessage, SourceSlice } from "../types";

export const chatSessions = $state<Record<string, ChatMessage[]>>({});
export const deepWikiSources = $state<Record<string, SourceSlice | null>>({});
export const knowledgeSources = $state<Record<string, SourceSlice | null>>({});

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
