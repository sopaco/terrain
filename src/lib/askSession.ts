import {
  clearActiveAskSession,
  createAskSession,
  discardAskSession,
  getActiveAskSession,
  listAskSessions,
  loadAskMessages,
  saveAskMessages,
  setActiveAskSession,
} from "./api";
import {
  activeAskSessionIds,
  chatSessions,
  setActiveAskSessionId,
  setAskSessions,
  updateChat,
} from "./stores/chat.svelte";
import type { ChatMessage } from "./types";

function hasAskContent(messages: ChatMessage[]): boolean {
  return messages.length > 0;
}

async function refreshAskSessions(projectSlug: string) {
  const sessions = await listAskSessions(projectSlug);
  setAskSessions(projectSlug, sessions);
  return sessions;
}

async function discardEmptySession(projectSlug: string, sessionId: string) {
  await discardAskSession(projectSlug, sessionId);
  await refreshAskSessions(projectSlug);
}

export async function loadAskProjectState(projectSlug: string): Promise<void> {
  const [sessions, activeId] = await Promise.all([
    listAskSessions(projectSlug),
    getActiveAskSession(projectSlug),
  ]);
  setAskSessions(projectSlug, sessions);

  if (activeId && !sessions.some((s) => s.id === activeId)) {
    await discardAskSession(projectSlug, activeId);
    await clearActiveAskSession(projectSlug);
  }

  const resolvedId =
    activeId && sessions.some((s) => s.id === activeId)
      ? activeId
      : (sessions[0]?.id ?? null);
  setActiveAskSessionId(projectSlug, resolvedId);

  if (resolvedId) {
    const messages = await loadAskMessages(projectSlug, resolvedId);
    updateChat(projectSlug, messages as ChatMessage[]);
  } else {
    updateChat(projectSlug, []);
  }
}

export async function switchAskSession(
  projectSlug: string,
  sessionId: string,
  currentMessages: ChatMessage[],
): Promise<void> {
  const prevId = activeAskSessionIds[projectSlug];
  if (prevId && prevId !== sessionId) {
    if (hasAskContent(currentMessages)) {
      const firstQuestion = currentMessages.find((m) => m.role === "user")?.content;
      await saveAskMessages(projectSlug, prevId, currentMessages, firstQuestion ?? null);
    } else {
      await discardEmptySession(projectSlug, prevId);
    }
  }
  const sessions = await setActiveAskSession(projectSlug, sessionId);
  setAskSessions(projectSlug, sessions);
  setActiveAskSessionId(projectSlug, sessionId);
  const messages = await loadAskMessages(projectSlug, sessionId);
  updateChat(projectSlug, messages as ChatMessage[]);
}

/** Start a blank draft — only persisted sessions with content appear in history. */
export async function startNewAskSession(projectSlug: string): Promise<void> {
  const prevId = activeAskSessionIds[projectSlug];
  const prevMessages = chatSessions[projectSlug] ?? [];

  if (prevId) {
    if (hasAskContent(prevMessages)) {
      const firstQuestion = prevMessages.find((m) => m.role === "user")?.content;
      await saveAskMessages(projectSlug, prevId, prevMessages, firstQuestion ?? null);
    } else {
      await discardEmptySession(projectSlug, prevId);
    }
  }

  await clearActiveAskSession(projectSlug);
  setActiveAskSessionId(projectSlug, null);
  updateChat(projectSlug, []);
  await refreshAskSessions(projectSlug);
}

export async function persistAskMessages(
  projectSlug: string,
  sessionId: string,
  messages: ChatMessage[],
) {
  const firstQuestion = messages.find((m) => m.role === "user")?.content;
  const updated = await saveAskMessages(
    projectSlug,
    sessionId,
    messages,
    firstQuestion ?? null,
  );
  await refreshAskSessions(projectSlug);
  return updated;
}

export async function ensureAskSessionForQuestion(
  projectSlug: string,
  question: string,
): Promise<string> {
  let sessionId = activeAskSessionIds[projectSlug];
  if (!sessionId) {
    const created = await createAskSession(projectSlug, question);
    await refreshAskSessions(projectSlug);
    setActiveAskSessionId(projectSlug, created.id);
    return created.id;
  }
  return sessionId;
}
