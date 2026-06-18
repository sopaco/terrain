<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount, tick } from "svelte";
  import { askKnowledge, readDocument } from "../api";
  import { parseAskSlashCommand } from "../askSlashCommands";
  import {
    appendStepText,
    finalizeAssistantSteps,
    finalAnswerFromSteps,
    startNewTextStep,
    syncStepTools,
  } from "../assistantSteps";
  import { isKnowledgeMarkdownPath } from "../knowledgeDoc";
  import { citationToSourceSlice, createPendingSourceSlice } from "../resolveSource";
  import { shouldSubmitOnEnter } from "../ime";
  import type {
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
  import { copyTextToClipboard } from "../clipboard";
  import { CHAT_PHASE_LABELS, UI_MESSAGES } from "../terminology";
  import CopyMarkdownButton from "./CopyMarkdownButton.svelte";
  import MarkdownViewer from "./MarkdownViewer.svelte";
  import SourcePanel from "./SourcePanel.svelte";
  import ToolCallTrace from "./ToolCallTrace.svelte";
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
  let listenersReady = $state(false);
  let activeRequestId = $state<string | null>(null);
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
  let copyToast = $state<{ text: string; ok: boolean } | null>(null);

  let copiedResetTimer: ReturnType<typeof setTimeout> | undefined;
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

  const streamCtx = { activeRequestId: null as string | null };

  function setActiveRequest(id: string | null) {
    activeRequestId = id;
    streamCtx.activeRequestId = id;
  }

  function sessionMatches(sessionId: string): boolean {
    return Boolean(
      streamCtx.activeRequestId && sessionId === streamCtx.activeRequestId,
    );
  }

  function resetStreamState() {
    streamSteps = [];
    streamingUsage = null;
    busy = false;
    streamPhase = "thinking";
    setActiveRequest(null);
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

  async function copyMarkdown(key: string, markdown: string) {
    const body = markdown.trim();
    if (!body || copyingMarkdownKey) return;

    copyingMarkdownKey = key;
    try {
      await copyTextToClipboard(body);
      markCopied(key);
      showCopyToast("已复制到剪贴板");
    } catch (e) {
      showCopyToast(`复制失败：${e}`, false);
    } finally {
      copyingMarkdownKey = null;
    }
  }

  const phaseLabel = $derived(
    streamPhase === "tools"
      ? CHAT_PHASE_LABELS.tools
      : streamPhase === "generating"
        ? CHAT_PHASE_LABELS.generating
        : streamPhase === "streaming"
          ? CHAT_PHASE_LABELS.streaming
          : CHAT_PHASE_LABELS.thinking,
  );

  const streamingUsageLine = $derived(formatUsageLine(streamingUsage));

  const streamScrollToken = $derived(
    streamSteps
      .map((step) =>
        step.kind === "text"
          ? step.content.length
          : step.toolCalls.map((c) => `${c.id}:${c.status}`).join(","),
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
      onmessageschange((prev) => [
        ...prev,
        {
          role: "assistant",
          content: finalContent,
          steps,
          citations: payload.citations,
          toolCalls: payload.toolCalls,
          timestamp: payload.completedAt ?? Date.now(),
          usage: payload.usage ?? streamingUsage ?? undefined,
        },
      ]);
    } catch (e) {
      console.error("Failed to save assistant message", e);
    } finally {
      turnCompleted = true;
      resetStreamState();
    }
  }

  $effect(() => {
    if (open && initialQuestion && !consumedInitial && !busy && listenersReady) {
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

  onMount(() => {
    let unlistenChunk: (() => void) | undefined;
    let unlistenTools: (() => void) | undefined;
    let unlistenPhase: (() => void) | undefined;
    let unlistenUsage: (() => void) | undefined;
    let unlistenDone: (() => void) | undefined;

    void (async () => {
      unlistenChunk = await listen<{ session_id: string; text: string }>(
        "chat-chunk",
        (ev) => {
          if (!sessionMatches(ev.payload.session_id)) return;
          streamSteps = appendStepText(streamSteps, ev.payload.text);
        },
      );
      unlistenTools = await listen<{
        session_id: string;
        tool_calls: ToolCallRecord[];
      }>("chat-tool-calls", (ev) => {
        if (!sessionMatches(ev.payload.session_id)) return;
        streamSteps = syncStepTools(streamSteps, ev.payload.tool_calls);
        if (ev.payload.tool_calls.some((call) => call.status === "running")) {
          streamPhase = "tools";
        }
      });
      unlistenPhase = await listen<{ session_id: string; phase: ChatPhase }>(
        "chat-phase",
        (ev) => {
          if (!sessionMatches(ev.payload.session_id)) return;
          streamPhase = ev.payload.phase;
          if (ev.payload.phase === "generating") {
            streamSteps = startNewTextStep(streamSteps);
          }
        },
      );
      unlistenUsage = await listen<{ session_id: string; usage: TokenUsage }>(
        "chat-usage",
        (ev) => {
          if (!sessionMatches(ev.payload.session_id)) return;
          streamingUsage = ev.payload.usage;
        },
      );
      unlistenDone = await listen<{
        session_id: string;
        answer: string;
        citations: SourceCitation[];
        tool_calls: ToolCallRecord[];
        usage: TokenUsage;
        completed_at: number;
      }>("chat-done", (ev) => {
        if (!sessionMatches(ev.payload.session_id)) return;
        streamingUsage = ev.payload.usage;
        completeAssistantTurn({
          answer: ev.payload.answer,
          citations: ev.payload.citations,
          toolCalls: ev.payload.tool_calls ?? [],
          usage: ev.payload.usage,
          completedAt: ev.payload.completed_at,
        });
      });
      listenersReady = true;
    })();

    return () => {
      unlistenChunk?.();
      unlistenTools?.();
      unlistenPhase?.();
      unlistenUsage?.();
      unlistenDone?.();
    };
  });

  function clearHistory() {
    if (busy) return;
    onmessageschange(() => []);
    showCopyToast("对话历史已清空");
  }

  async function sendQuestion(q: string) {
    if (!q.trim() || busy || !projectSlug) return;

    const slash = parseAskSlashCommand(q);
    if (slash?.type === "clear") {
      clearHistory();
      input = "";
      return;
    }

    const question = q.trim();
    const requestId = crypto.randomUUID();
    const requestRepo = repoPath;

    onmessageschange((prev) => [
      ...prev,
      { role: "user", content: question, timestamp: Date.now() },
    ]);
    input = "";
    busy = true;
    stickToBottom = true;
    turnCompleted = false;
    streamPhase = "thinking";
    setActiveRequest(requestId);
    streamSteps = [];
    streamingUsage = null;

    try {
      const reply = await askKnowledge(
        question,
        projectSlug,
        requestRepo ?? undefined,
        requestId,
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
    <div class="relative flex w-[min(760px,92vw)] shrink-0 flex-col border-l border-white/10 bg-[#12151c] shadow-2xl">
    {#if copyToast}
      <div
        class="pointer-events-none absolute inset-x-0 top-3 z-[70] flex justify-center px-4"
        role="status"
        aria-live="polite"
      >
        <div
          class="copy-toast flex items-center gap-2 rounded-xl border px-4 py-2.5 text-sm font-medium shadow-2xl backdrop-blur-md {copyToast.ok
            ? 'border-emerald-400/35 bg-emerald-500/20 text-emerald-100'
            : 'border-red-400/35 bg-red-500/20 text-red-100'}"
        >
          {#if copyToast.ok}
            <svg class="h-4 w-4 shrink-0" viewBox="0 0 16 16" fill="none" aria-hidden="true">
              <path
                d="M3.5 8.5L6.5 11.5L12.5 4.5"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          {/if}
          {copyToast.text}
        </div>
      </div>
    {/if}
    <header class="flex shrink-0 items-center gap-3 border-b border-white/10 px-4 py-3">
      <div class="min-w-0 flex-1">
        <h2 class="text-sm font-semibold">DeepWiki</h2>
        <p class="truncate text-xs text-white/40">
          {projectName ?? projectSlug ?? UI_MESSAGES.noProject} · 可继续追问
        </p>
      </div>
      {#if busy}
        <span class="rounded-full bg-indigo-500/20 px-2 py-1 text-xs text-indigo-200">
          {phaseLabel}
        </span>
      {/if}
      <button
        type="button"
        class="rounded-lg border border-white/10 px-3 py-1.5 text-sm text-white/70 hover:bg-white/5 disabled:cursor-not-allowed disabled:opacity-40"
        disabled={busy || messages.length === 0}
        onclick={clearHistory}
      >
        Clear
      </button>
      <button
        type="button"
        class="rounded-lg border border-white/10 px-3 py-1.5 text-sm text-white/70 hover:bg-white/5"
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
                msg.role === "user" ? "bg-indigo-600/25" : "bg-white/[0.04]"
              }`}
            >
              <div class="mb-2 flex items-center justify-between gap-2 text-[10px] uppercase tracking-wide text-white/40">
                <span>{msg.role === "user" ? "You" : "MindMesh"}</span>
                {#if msg.timestamp}
                  <span class="normal-case text-white/30">{formatTime(msg.timestamp)}</span>
                {/if}
              </div>
              {#if msg.role === "user"}
                <p class="whitespace-pre-wrap text-sm leading-relaxed text-white/90">{msg.content}</p>
              {:else}
                {#if msg.steps?.length}
                  {#each msg.steps as step, i (i)}
                    {#if step.kind === "tools"}
                      <div class="my-3">
                        <ToolCallTrace toolCalls={step.toolCalls} />
                      </div>
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
                <div class="mt-3 flex items-center justify-between gap-2 border-t border-white/5 pt-2">
                  {#if formatUsageLine(msg.usage)}
                    <p class="text-[10px] text-white/35">{formatUsageLine(msg.usage)}</p>
                  {:else}
                    <span></span>
                  {/if}
                  {#if msg.role === "assistant" && assistantMessageMarkdown(msg)}
                    {@const copyKey = `done-${msg.timestamp ?? i}`}
                    <CopyMarkdownButton
                      copied={copiedMarkdownKey === copyKey}
                      copying={copyingMarkdownKey === copyKey}
                      onclick={() => void copyMarkdown(copyKey, assistantMessageMarkdown(msg))}
                    />
                  {/if}
                </div>
              {/if}
              {#if msg.citations?.length}
                <div class="mt-4 space-y-1.5 border-t border-white/5 pt-3">
                  <p class="text-[10px] uppercase tracking-wide text-white/35">Sources</p>
                  {#each msg.citations as c}
                    <button
                      type="button"
                      class="block w-full rounded-lg border border-white/10 bg-black/20 px-3 py-2 text-left text-xs hover:bg-white/5"
                      onclick={() => openCitation(c)}
                    >
                      <span class="text-white/40">{c.kind}</span>
                      <div class="font-medium text-indigo-200">{c.title}</div>
                      {#if c.excerpt}
                        <p class="mt-0.5 line-clamp-2 text-white/50">{c.excerpt}</p>
                      {/if}
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          {/each}

          {#if busy}
            <div class="rounded-xl bg-white/[0.04] px-4 py-3">
              <div class="mb-2 flex items-center justify-between gap-2 text-[10px] uppercase tracking-wide text-white/40">
                <span>MindMesh</span>
                <span class="normal-case text-indigo-200/80">{phaseLabel}</span>
              </div>
              {#if streamSteps.length === 0}
                <div class="mb-3 flex items-center gap-2">
                  <span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-indigo-300 border-t-transparent"></span>
                  <span class="text-xs text-white/50">{phaseLabel}</span>
                </div>
              {:else}
                {#each streamSteps as step, i (i)}
                  {#if step.kind === "tools"}
                    <div class="my-3">
                      <ToolCallTrace toolCalls={step.toolCalls} defaultExpanded={i === streamSteps.length - 1} />
                    </div>
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
                <div class="mt-3 flex items-center justify-between gap-2 border-t border-white/5 pt-2">
                  {#if streamingUsageLine}
                    <p class="text-[10px] text-white/35">{streamingUsageLine}</p>
                  {:else}
                    <span></span>
                  {/if}
                  {#if assistantStepsMarkdown(streamSteps)}
                    <CopyMarkdownButton
                      copied={copiedMarkdownKey === "streaming"}
                      copying={copyingMarkdownKey === "streaming"}
                      onclick={() => void copyMarkdown("streaming", assistantStepsMarkdown(streamSteps))}
                    />
                  {/if}
                </div>
              {/if}
            </div>
          {:else if messages.length === 0}
            <p class="text-center text-sm text-white/35">
              Ask about architecture, workflows, or APIs. Citations appear below each answer.
            </p>
          {/if}
          {#if citationError}
            <p class="rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-200">
              {citationError}
            </p>
          {/if}
          </div>
        </div>

        <div class="shrink-0 border-t border-white/10 p-4">
          <textarea
            class="mb-2 w-full resize-none rounded-xl border border-white/10 bg-white/5 px-3 py-2 text-sm outline-none focus:border-indigo-500"
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
            class="w-full rounded-xl bg-indigo-600 py-2.5 text-sm font-medium hover:bg-indigo-500 disabled:opacity-50"
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
          class="mm-source-rail-panel flex flex-col bg-[#10131a]"
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
