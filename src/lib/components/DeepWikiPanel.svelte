<script lang="ts">
  import { tick } from "svelte";
  import { Check, MessageSquarePlus } from "@lucide/svelte";
  import { askKnowledge, deleteAskSession, loadAskMessages, readDocument } from "../api";
  import {
    ensureAskSessionForQuestion,
    persistAskMessages,
    startNewAskSession,
    switchAskSession,
  } from "../askSession";
  import { parseAskSlashCommand } from "../askSlashCommands";
  import {
    appendStepText,
    appendStepThinking,
    finalizeAssistantSteps,
    finalAnswerFromSteps,
    isThinkingStep,
    isThinkingStepActive,
    startNewTextStep,
    syncStepTools,
  } from "../assistantSteps";
  import { isKnowledgeMarkdownPath } from "../knowledgeDoc";
  import { citationToSourceSlice, createPendingSourceSlice } from "../resolveSource";
  import { shouldSubmitOnEnter } from "../ime";
  import type {
    AskStreamEvent,
    AssistantStep,
    ChatMessage,
    ChatPhase,
    SourceCitation,
    SourceSlice,
    TokenUsage,
    ToolCallRecord,
  } from "../types";
  import { formatTime } from "../timeFormat";
  import { assistantMessageMarkdown, assistantStepsMarkdown } from "../assistantMarkdown";
  import { renderAskSharePng, formatUnknownError } from "../askShareImage";
  import { copyPngBlobToClipboard, copyTextToClipboard } from "../clipboard";
  import { CHAT_PHASE_LABELS, UI_MESSAGES } from "../terminology";
  import CopyImageButton from "./CopyImageButton.svelte";
  import CopyMarkdownButton from "./CopyMarkdownButton.svelte";
  import MarkdownViewer from "./MarkdownViewer.svelte";
  import SourcePanel from "./SourcePanel.svelte";
  import ToolCallTrace from "./ToolCallTrace.svelte";
  import ThinkingTrace from "./ThinkingTrace.svelte";
  import AskSessionSelector from "./AskSessionSelector.svelte";
  import {
    activeAskSessionIds,
    askSessionLists,
    setActiveAskSessionId,
    setAskSessions,
    showAskCompletionNotice,
  } from "../stores/chat.svelte";
  import {
    handleDismissTransitionEnd,
    prefersReducedMotion,
    schedulePresent,
  } from "../overlayMotion";

  type MessageUpdater = ChatMessage[] | ((prev: ChatMessage[]) => ChatMessage[]);

  interface Props {
    open: boolean;
    projectSlug: string | null;
    projectName?: string | null;
    repoPath: string | null;
    messages: ChatMessage[];
    initialQuestion?: string | null;
    sourceSlice: SourceSlice | null;
    onclose: () => void;
    onmessageschange: (update: MessageUpdater) => void;
    onsourcechange: (slice: SourceSlice | null) => void;
    onopenDoc: (path: string) => void;
  }

  let {
    open,
    projectSlug,
    projectName = null,
    repoPath,
    messages,
    initialQuestion = null,
    sourceSlice,
    onclose,
    onmessageschange,
    onsourcechange,
    onopenDoc,
  }: Props = $props();

  let input = $state("");
  let busy = $state(false);
  let streamSteps = $state<AssistantStep[]>([]);
  let streamingUsage = $state<TokenUsage | null>(null);
  let turnCompleted = $state(false);
  let messagesScrollEl = $state<HTMLDivElement | null>(null);
  let messagesContentEl = $state<HTMLDivElement | null>(null);
  let stickToBottom = $state(true);
  let consumedInitial = $state(false);
  let citationError = $state<string | null>(null);
  let composing = $state(false);
  let streamPhase = $state<ChatPhase>("thinking");
  let copyingMarkdownKey = $state<string | null>(null);
  let copiedMarkdownKey = $state<string | null>(null);
  let copyingImageKey = $state<string | null>(null);
  let copiedImageKey = $state<string | null>(null);
  let copyToast = $state<{ text: string; ok: boolean } | null>(null);
  let sessionMenuOpen = $state(false);
  let sessionBusy = $state(false);
  let currentSessionId = $state<string | null>(null);

  const askSessions = $derived(
    projectSlug ? (askSessionLists[projectSlug] ?? []) : [],
  );
  const activeSessionId = $derived(
    projectSlug ? (activeAskSessionIds[projectSlug] ?? null) : null,
  );

  $effect(() => {
    currentSessionId = activeSessionId;
  });

  let copiedResetTimer: ReturnType<typeof setTimeout> | undefined;
  let copiedImageResetTimer: ReturnType<typeof setTimeout> | undefined;
  let copyToastTimer: ReturnType<typeof setTimeout> | undefined;

  const sourceOpen = $derived(Boolean(sourceSlice));

  let mounted = $state(false);
  let presented = $state(false);
  let showChat = $state(false);

  let sourceRailMounted = $state(false);
  let sourceRailPresented = $state(false);
  let showSourcePanel = $state(false);
  let visibleSourceSlice = $state<SourceSlice | null>(null);
  let sourceLoadId = 0;

  $effect(() => {
    if (open) {
      mounted = true;
      showChat = true;
      void schedulePresent((value) => {
        presented = value;
      });
      return;
    }
    presented = false;
    if (prefersReducedMotion()) {
      showChat = false;
      mounted = false;
    }
  });

  $effect(() => {
    if (sourceSlice) visibleSourceSlice = sourceSlice;
  });

  $effect(() => {
    if (sourceSlice) {
      if (!sourceRailMounted) {
        sourceRailMounted = true;
        showSourcePanel = false;
        void schedulePresent((value) => {
          sourceRailPresented = value;
          if (value) showSourcePanel = true;
        });
      } else {
        sourceRailPresented = true;
        showSourcePanel = true;
      }
      return;
    }
    sourceRailPresented = false;
    if (prefersReducedMotion()) {
      showSourcePanel = false;
      sourceRailMounted = false;
      visibleSourceSlice = null;
    }
  });

  function onDrawerTransitionEnd(e: TransitionEvent) {
    handleDismissTransitionEnd(e, presented, () => {
      showChat = false;
      mounted = false;
    });
  }

  function onSourceRailTransitionEnd(e: TransitionEvent) {
    handleDismissTransitionEnd(e, sourceRailPresented, () => {
      showSourcePanel = false;
      sourceRailMounted = false;
      if (!sourceSlice) visibleSourceSlice = null;
    });
  }

  function onWindowKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && open) onclose();
  }

  function resetStreamState() {
    streamSteps = [];
    streamingUsage = null;
    busy = false;
    streamPhase = "thinking";
  }

  function handleAskStreamEvent(event: AskStreamEvent) {
    switch (event.type) {
      case "chunk":
        streamSteps = appendStepText(streamSteps, event.text);
        break;
      case "thinking_chunk":
        streamSteps = appendStepThinking(streamSteps, event.text);
        break;
      case "tool_calls":
        streamSteps = syncStepTools(streamSteps, event.tool_calls);
        if (event.tool_calls.some((call) => call.status === "running")) {
          streamPhase = "tools";
        }
        break;
      case "phase":
        streamPhase = event.phase;
        if (event.phase === "generating") {
          streamSteps = startNewTextStep(streamSteps);
        }
        break;
      case "usage":
        streamingUsage = event.usage;
        break;
      case "done":
        streamingUsage = event.reply.usage;
        completeAssistantTurn({
          answer: event.reply.answer,
          citations: event.reply.citations,
          toolCalls: event.reply.tool_calls ?? [],
          usage: event.reply.usage,
          completedAt: event.reply.completed_at,
        });
        break;
    }
  }

  function formatUsageLine(usage?: TokenUsage | null): string | null {
    if (!usage) return null;
    if (usage.input_tokens === 0 && usage.output_tokens === 0) return null;
    const suffix = usage.estimated ? " (estimated)" : "";
    return `Tokens — input ${usage.input_tokens.toLocaleString()} · output ${usage.output_tokens.toLocaleString()}${suffix}`;
  }

  function showCopyToast(text: string, ok = true) {
    clearTimeout(copyToastTimer);
    copyToast = { text, ok };
    copyToastTimer = setTimeout(() => {
      copyToast = null;
    }, 2600);
  }

  function markCopied(key: string) {
    clearTimeout(copiedResetTimer);
    copiedMarkdownKey = key;
    copiedResetTimer = setTimeout(() => {
      if (copiedMarkdownKey === key) {
        copiedMarkdownKey = null;
      }
    }, 2200);
  }

  function markImageCopied(key: string) {
    clearTimeout(copiedImageResetTimer);
    copiedImageKey = key;
    copiedImageResetTimer = setTimeout(() => {
      if (copiedImageKey === key) {
        copiedImageKey = null;
      }
    }, 2200);
  }

  function questionBeforeIndex(index: number): string | null {
    for (let j = index - 1; j >= 0; j -= 1) {
      if (messages[j].role === "user") return messages[j].content.trim();
    }
    return null;
  }

  function lastUserQuestion(): string | null {
    for (let j = messages.length - 1; j >= 0; j -= 1) {
      if (messages[j].role === "user") return messages[j].content.trim();
    }
    return null;
  }

  async function copyMarkdown(key: string, markdown: string) {
    const body = markdown.trim();
    if (!body || copyingMarkdownKey) return;

    copyingMarkdownKey = key;
    try {
      await copyTextToClipboard(body);
      markCopied(key);
      showCopyToast("已复制到剪贴板");
    } catch (e) {
      showCopyToast(`复制失败：${formatUnknownError(e)}`, false);
    } finally {
      copyingMarkdownKey = null;
    }
  }

  async function copyImage(key: string, question: string | null, answerMarkdown: string) {
    const q = question?.trim();
    const answer = answerMarkdown.trim();
    if (!q || !answer || copyingImageKey) return;

    copyingImageKey = key;
    try {
      const blob = await renderAskSharePng({
        question: q,
        answerMarkdown: answer,
        projectName: projectName ?? projectSlug,
      });
      await copyPngBlobToClipboard(blob);
      markImageCopied(key);
      showCopyToast("图片已复制到剪贴板");
    } catch (e) {
      showCopyToast(`复制失败：${formatUnknownError(e)}`, false);
    } finally {
      copyingImageKey = null;
    }
  }

  const phaseLabel = $derived(CHAT_PHASE_LABELS[streamPhase] ?? CHAT_PHASE_LABELS.thinking);

  const streamingUsageLine = $derived(formatUsageLine(streamingUsage));

  const streamScrollToken = $derived(
    streamSteps
      .map((step) =>
        step.kind === "text" || step.kind === "thinking"
          ? step.content.length
          : step.kind === "tools"
            ? step.toolCalls.map((c) => `${c.id}:${c.status}`).join(",")
            : "",
      )
      .join("|"),
  );

  function isNearBottom(el: HTMLElement, threshold = 96): boolean {
    return el.scrollHeight - el.scrollTop - el.clientHeight <= threshold;
  }

  function scrollToBottom(instant = false) {
    const el = messagesScrollEl;
    if (!el) return;
    el.scrollTo({ top: el.scrollHeight, behavior: instant ? "auto" : "smooth" });
  }

  function scrollToBottomIfNeeded(force = false) {
    if (!force && busy && !stickToBottom) return;
    scrollToBottom(busy || force);
  }

  function onMessagesScroll() {
    const el = messagesScrollEl;
    if (!el) return;
    stickToBottom = isNearBottom(el);
  }

  function sessionTitle(): string {
    if (!projectSlug || !currentSessionId) return lastUserQuestion()?.slice(0, 10) ?? "Ask";
    const session = askSessions.find((s) => s.id === currentSessionId);
    return session?.title ?? lastUserQuestion()?.slice(0, 10) ?? "Ask";
  }

  async function persistCurrentMessages(nextMessages: ChatMessage[]) {
    if (!projectSlug || !currentSessionId) return;
    try {
      await persistAskMessages(projectSlug, currentSessionId, nextMessages);
    } catch (e) {
      console.error("Failed to persist ask messages", e);
    }
  }

  function completeAssistantTurn(payload: {
    answer: string;
    citations: SourceCitation[];
    toolCalls: ToolCallRecord[];
    usage?: TokenUsage;
    completedAt?: number;
  }) {
    if (turnCompleted) return;

    const steps = finalizeAssistantSteps(streamSteps, payload.answer, payload.toolCalls);
    const finalContent =
      finalAnswerFromSteps(steps, payload.answer) ||
      "已完成知识库检索，但未收到最终文本回复。请查看上方工具调用结果。"

    try {
      const nextMessages: ChatMessage[] = [
        ...messages,
        {
          role: "assistant",
          content: finalContent,
          steps,
          citations: payload.citations,
          toolCalls: payload.toolCalls,
          timestamp: payload.completedAt ?? Date.now(),
          usage: payload.usage ?? streamingUsage ?? undefined,
        },
      ];
      onmessageschange(() => nextMessages);
      void persistCurrentMessages(nextMessages);

      if (!open && projectSlug && currentSessionId) {
        showAskCompletionNotice({
          projectSlug,
          sessionId: currentSessionId,
          title: sessionTitle(),
          answerMarkdown: finalContent,
        });
      }
    } catch (e) {
      console.error("Failed to save assistant message", e);
    } finally {
      turnCompleted = true;
      resetStreamState();
    }
  }

  $effect(() => {
    if (open && initialQuestion && !consumedInitial && !busy) {
      consumedInitial = true;
      void sendQuestion(initialQuestion);
    }
    if (!open) {
      consumedInitial = false;
    }
  });

  $effect(() => {
    if (!open) return;
    messages.length;
    streamScrollToken;
    busy;

    void (async () => {
      await tick();
      scrollToBottomIfNeeded(!busy);
    })();
  });

  $effect(() => {
    const content = messagesContentEl;
    const scrollEl = messagesScrollEl;
    if (!content || !scrollEl) return;

    const observer = new ResizeObserver(() => {
      if (busy && stickToBottom) {
        scrollEl.scrollTop = scrollEl.scrollHeight;
      }
    });
    observer.observe(content);
    return () => observer.disconnect();
  });

  async function handleNewSession() {
    if (!projectSlug || busy || sessionBusy) return;
    sessionBusy = true;
    try {
      await startNewAskSession(projectSlug);
      sessionMenuOpen = false;
      showCopyToast("已新建对话");
    } catch (e) {
      showCopyToast(`新建对话失败：${e}`, false);
    } finally {
      sessionBusy = false;
    }
  }

  async function handleSelectSession(sessionId: string) {
    if (!projectSlug || busy || sessionId === currentSessionId) {
      sessionMenuOpen = false;
      return;
    }
    sessionBusy = true;
    try {
      await switchAskSession(projectSlug, sessionId, messages);
      sessionMenuOpen = false;
    } catch (e) {
      showCopyToast(`切换对话失败：${e}`, false);
    } finally {
      sessionBusy = false;
    }
  }

  async function handleDeleteSession(sessionId: string) {
    if (!projectSlug || busy) return;
    sessionBusy = true;
    try {
      const sessions = await deleteAskSession(projectSlug, sessionId);
      setAskSessions(projectSlug, sessions);
      if (currentSessionId === sessionId) {
        const next = sessions[0]?.id ?? null;
        setActiveAskSessionId(projectSlug, next);
        if (next) {
          const loaded = await loadAskMessages(projectSlug, next);
          onmessageschange(() => loaded);
        } else {
          onmessageschange(() => []);
        }
      }
      sessionMenuOpen = false;
    } catch (e) {
      showCopyToast(`删除对话失败：${e}`, false);
    } finally {
      sessionBusy = false;
    }
  }

  async function sendQuestion(q: string) {
    if (!q.trim() || busy || !projectSlug) return;

    const slash = parseAskSlashCommand(q);
    if (slash?.type === "new") {
      await handleNewSession();
      input = "";
      return;
    }

    const question = q.trim();
    const requestRepo = repoPath;

    if (!projectSlug) return;

    let sessionId = currentSessionId;
    if (!sessionId) {
      sessionId = await ensureAskSessionForQuestion(projectSlug, question);
      currentSessionId = sessionId;
    }

    const userMessages: ChatMessage[] = [
      ...messages,
      { role: "user", content: question, timestamp: Date.now() },
    ];
    onmessageschange(() => userMessages);
    void persistAskMessages(projectSlug, sessionId, userMessages);
    input = "";
    busy = true;
    stickToBottom = true;
    turnCompleted = false;
    streamPhase = "thinking";
    streamSteps = [];
    streamingUsage = null;

    try {
      const reply = await askKnowledge(
        question,
        projectSlug,
        requestRepo ?? undefined,
        handleAskStreamEvent,
      );
      if (!turnCompleted) {
        completeAssistantTurn({
          answer: reply.answer,
          citations: reply.citations,
          toolCalls: reply.tool_calls ?? [],
          usage: reply.usage,
          completedAt: reply.completed_at,
        });
      }
    } catch (e) {
      if (!turnCompleted) {
        try {
          onmessageschange((prev) => [
            ...prev,
            { role: "assistant", content: `错误：${e}`, timestamp: Date.now() },
          ]);
        } catch {
          // ignore persistence errors
        }
        turnCompleted = true;
      }
    } finally {
      if (busy) {
        const draftAnswer = finalAnswerFromSteps(streamSteps, "");
        if (!turnCompleted && draftAnswer) {
          completeAssistantTurn({
            answer: draftAnswer,
            citations: [],
            toolCalls: streamSteps
              .filter((s) => s.kind === "tools")
              .flatMap((s) => (s.kind === "tools" ? s.toolCalls : [])),
          });
        } else {
          resetStreamState();
          if (!turnCompleted) {
            turnCompleted = true;
          }
        }
      }
    }
  }

  async function send() {
    await sendQuestion(input);
  }

  async function openCitation(c: SourceCitation) {
    citationError = null;

    if (!projectSlug && !isKnowledgeMarkdownPath(c.path)) {
      citationError = UI_MESSAGES.noProjectSelected;
      return;
    }

    const loadId = ++sourceLoadId;
    onsourcechange(createPendingSourceSlice(c, repoPath));

    try {
      const slice = await citationToSourceSlice(
        projectSlug ?? "",
        c,
        repoPath,
        readDocument,
      );
      if (loadId !== sourceLoadId) return;
      onsourcechange(slice);
    } catch (e) {
      if (loadId !== sourceLoadId) return;
      citationError = String(e);
      onsourcechange({
        ...createPendingSourceSlice(c, repoPath),
        status: "error",
        format: "markdown",
        content: String(e),
      });
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (shouldSubmitOnEnter(e) && !composing) {
      e.preventDefault();
      void send();
    }
    if (e.key === "Escape") {
      if (sessionMenuOpen) {
        sessionMenuOpen = false;
        return;
      }
      onclose();
    }
  }
</script>

<style>
  @keyframes copy-toast-in {
    from {
      opacity: 0;
      transform: translateY(-10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .copy-toast {
    animation: copy-toast-in 180ms ease-out;
  }
</style>

{#if mounted}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="mm-overlay-backdrop fixed inset-0 z-40 bg-black/55"
    class:is-presented={presented}
    onclick={onclose}
    role="presentation"
  ></div>

  <section
    class="mm-overlay-drawer fixed inset-y-0 right-0 z-50 flex flex-row-reverse items-stretch"
    class:is-presented={presented}
    aria-label="DeepWiki"
    ontransitionend={onDrawerTransitionEnd}
  >
    {#if showChat}
    <div class="relative flex w-[min(760px,92vw)] shrink-0 flex-col border-l border-tr-border-strong bg-tr-surface shadow-2xl">
    {#if copyToast}
      <div
        class="pointer-events-none absolute inset-x-0 top-3 z-[70] flex justify-center px-4"
        role="status"
        aria-live="polite"
      >
        <div
          class="copy-toast flex items-center gap-2 rounded-xl border px-4 py-2.5 text-sm font-medium shadow-2xl backdrop-blur-md {copyToast.ok
            ? 'border-tr-good/35 bg-tr-good-soft text-tr-good'
            : 'border-tr-critical/35 bg-tr-critical/20 text-tr-on-critical'}"
        >
          {#if copyToast.ok}
            <Check size={16} strokeWidth={2.5} class="shrink-0" aria-hidden="true" />
          {/if}
          {copyToast.text}
        </div>
      </div>
    {/if}
    <header class="flex shrink-0 items-center gap-3 border-b border-tr-border-strong px-4 py-3">
      <div class="min-w-0 flex-1">
        <h2 class="text-sm font-semibold">DeepWiki</h2>
        <p class="truncate text-xs text-tr-ink-3">
          {projectName ?? projectSlug ?? UI_MESSAGES.noProject} · 可继续追问
        </p>
      </div>
      {#if busy}
        <span class="rounded-full bg-tr-accent-soft-strong px-2 py-1 text-xs text-tr-accent">
          {phaseLabel}
        </span>
      {/if}
      <button
        type="button"
        class="inline-flex h-9 w-9 items-center justify-center rounded-lg border border-tr-border-strong text-tr-ink-2 transition-colors hover:bg-tr-elevated disabled:cursor-not-allowed disabled:opacity-40"
        disabled={busy || sessionBusy || !projectSlug}
        onclick={() => void handleNewSession()}
        aria-label="新建对话"
        title="新建对话"
      >
        <MessageSquarePlus size={16} strokeWidth={2} aria-hidden="true" />
      </button>
      {#if projectSlug}
        <AskSessionSelector
          sessions={askSessions}
          activeSessionId={currentSessionId}
          open={sessionMenuOpen}
          creating={sessionBusy}
          ontoggle={() => (sessionMenuOpen = !sessionMenuOpen)}
          onselect={handleSelectSession}
          oncreate={handleNewSession}
          ondelete={handleDeleteSession}
        />
      {/if}
      <button
        type="button"
        class="rounded-lg border border-tr-border-strong px-3 py-1.5 text-sm text-tr-ink-2 hover:bg-tr-elevated"
        onclick={onclose}
      >
        Close
      </button>
    </header>

    <div class="flex min-h-0 flex-1 flex-col overflow-hidden">
        <div
          bind:this={messagesScrollEl}
          class="flex-1 overflow-y-auto p-4"
          onscroll={onMessagesScroll}
        >
          <div bind:this={messagesContentEl} class="space-y-4">
          {#each messages as msg, i (msg.timestamp ?? `msg-${i}`)}
            <div
              class={`rounded-xl px-4 py-3 ${
                msg.role === "user" ? "bg-tr-accent/25" : "bg-tr-elevated"
              }`}
            >
              <div class="mb-2 flex items-center justify-between gap-2 text-[10px] uppercase tracking-wide text-tr-ink-3">
                <span>{msg.role === "user" ? "You" : "Terrain"}</span>
                {#if msg.timestamp}
                  <span class="normal-case text-tr-ink-3">{formatTime(msg.timestamp)}</span>
                {/if}
              </div>
              {#if msg.role === "user"}
                <p class="whitespace-pre-wrap text-sm leading-relaxed text-tr-ink">{msg.content}</p>
              {:else}
                {#if msg.steps?.length}
                  {#each msg.steps as step, i (i)}
                    {#if step.kind === "tools"}
                      <div class="my-3">
                        <ToolCallTrace toolCalls={step.toolCalls} />
                      </div>
                    {:else if isThinkingStep(msg.steps, i)}
                      <ThinkingTrace content={step.content} {repoPath} />
                    {:else if step.content.trim()}
                      <MarkdownViewer body={step.content} repoPath={repoPath} compact onSourceClick={openCitation} />
                    {/if}
                  {/each}
                {:else}
                  {#if msg.toolCalls?.length}
                    <div class="mb-3">
                      <ToolCallTrace toolCalls={msg.toolCalls} />
                    </div>
                  {/if}
                  <MarkdownViewer body={msg.content} repoPath={repoPath} compact onSourceClick={openCitation} />
                {/if}
              {/if}
              {#if formatUsageLine(msg.usage) || (msg.role === "assistant" && assistantMessageMarkdown(msg))}
                <div class="mt-3 flex items-center justify-between gap-2 border-t border-tr-border pt-2">
                  {#if formatUsageLine(msg.usage)}
                    <p class="text-[10px] text-tr-ink-3">{formatUsageLine(msg.usage)}</p>
                  {:else}
                    <span></span>
                  {/if}
                  {#if msg.role === "assistant" && assistantMessageMarkdown(msg)}
                    {@const copyKey = `done-${msg.timestamp ?? i}`}
                    <div class="flex shrink-0 items-center gap-1.5">
                      <CopyImageButton
                        copied={copiedImageKey === copyKey}
                        copying={copyingImageKey === copyKey}
                        onclick={() =>
                          void copyImage(copyKey, questionBeforeIndex(i), assistantMessageMarkdown(msg))}
                      />
                      <CopyMarkdownButton
                        copied={copiedMarkdownKey === copyKey}
                        copying={copyingMarkdownKey === copyKey}
                        onclick={() => void copyMarkdown(copyKey, assistantMessageMarkdown(msg))}
                      />
                    </div>
                  {/if}
                </div>
              {/if}
              {#if msg.citations?.length}
                <div class="mt-4 space-y-1.5 border-t border-tr-border pt-3">
                  <p class="text-[10px] uppercase tracking-wide text-tr-ink-3">Sources</p>
                  {#each msg.citations as c}
                    <button
                      type="button"
                      class="block w-full rounded-lg border border-tr-border-strong bg-tr-page px-3 py-2 text-left text-xs hover:bg-tr-elevated"
                      onclick={() => openCitation(c)}
                    >
                      <span class="text-tr-ink-3">{c.kind}</span>
                      <div class="font-medium text-tr-accent">{c.title}</div>
                      {#if c.excerpt}
                        <p class="mt-0.5 line-clamp-2 text-tr-ink-3">{c.excerpt}</p>
                      {/if}
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          {/each}

          {#if busy}
            <div class="rounded-xl bg-tr-elevated px-4 py-3">
              <div class="mb-2 flex items-center justify-between gap-2 text-[10px] uppercase tracking-wide text-tr-ink-3">
                <span>Terrain</span>
                <span class="normal-case text-tr-accent">{phaseLabel}</span>
              </div>
              {#if streamSteps.length === 0}
                <div class="mb-3 flex items-center gap-2">
                  <span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-tr-accent border-t-transparent"></span>
                  <span class="text-xs text-tr-ink-3">{phaseLabel}</span>
                </div>
              {:else}
                {#each streamSteps as step, i (i)}
                  {#if step.kind === "tools"}
                    <div class="my-3">
                      <ToolCallTrace toolCalls={step.toolCalls} defaultExpanded={i === streamSteps.length - 1} />
                    </div>
                  {:else if isThinkingStep(streamSteps, i)}
                    <ThinkingTrace
                      content={step.content}
                      active={isThinkingStepActive(streamSteps, i, busy)}
                      {repoPath}
                    />
                  {:else if step.content.trim()}
                    <MarkdownViewer
                      body={step.content}
                      repoPath={repoPath}
                      compact
                      allowMermaid={false}
                      onSourceClick={openCitation}
                    />
                  {/if}
                {/each}
              {/if}
              {#if streamingUsageLine || assistantStepsMarkdown(streamSteps)}
                <div class="mt-3 flex items-center justify-between gap-2 border-t border-tr-border pt-2">
                  {#if streamingUsageLine}
                    <p class="text-[10px] text-tr-ink-3">{streamingUsageLine}</p>
                  {:else}
                    <span></span>
                  {/if}
                  {#if assistantStepsMarkdown(streamSteps)}
                    <div class="flex shrink-0 items-center gap-1.5">
                      <CopyImageButton
                        copied={copiedImageKey === "streaming"}
                        copying={copyingImageKey === "streaming"}
                        onclick={() =>
                          void copyImage(
                            "streaming",
                            lastUserQuestion(),
                            assistantStepsMarkdown(streamSteps),
                          )}
                      />
                      <CopyMarkdownButton
                        copied={copiedMarkdownKey === "streaming"}
                        copying={copyingMarkdownKey === "streaming"}
                        onclick={() => void copyMarkdown("streaming", assistantStepsMarkdown(streamSteps))}
                      />
                    </div>
                  {/if}
                </div>
              {/if}
            </div>
          {:else if messages.length === 0}
            <p class="text-center text-sm text-tr-ink-3">
              Ask about architecture, workflows, or APIs. Citations appear below each answer.
            </p>
          {/if}
          {#if citationError}
            <p class="rounded-lg border border-tr-critical/30 bg-tr-critical-soft px-3 py-2 text-xs text-tr-critical">
              {citationError}
            </p>
          {/if}
          </div>
        </div>

        <div class="shrink-0 border-t border-tr-border-strong p-4">
          <textarea
            class="mb-2 w-full resize-none rounded-xl border border-tr-border-strong bg-tr-elevated px-3 py-2 text-sm outline-none focus:border-tr-accent"
            rows="2"
            placeholder={UI_MESSAGES.askFollowUpPlaceholder}
            bind:value={input}
            onkeydown={onKeydown}
            oncompositionstart={() => (composing = true)}
            oncompositionend={() => (composing = false)}
            disabled={!projectSlug || busy}
          ></textarea>
          <button
            type="button"
            class="w-full rounded-xl bg-tr-accent py-2.5 text-sm font-medium hover:bg-tr-accent-hover disabled:opacity-50"
            disabled={!projectSlug || busy}
            onclick={send}
          >
            {busy ? phaseLabel : "Send"}
          </button>
        </div>
    </div>
    </div>
    {/if}

    {#if sourceRailMounted && visibleSourceSlice}
      <aside class="mm-source-rail" aria-hidden={!sourceOpen}>
        <div
          class="mm-source-rail-panel flex flex-col bg-tr-page"
          class:is-presented={sourceRailPresented}
          ontransitionend={onSourceRailTransitionEnd}
        >
          {#if showSourcePanel}
            <SourcePanel
              slice={visibleSourceSlice}
              {repoPath}
              onclose={() => onsourcechange(null)}
              onSourceClick={openCitation}
            />
          {/if}
        </div>
      </aside>
    {/if}
  </section>
{/if}

<svelte:window onkeydown={onWindowKeydown} />
