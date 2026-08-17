<script lang="ts">
  import { ChevronDown, ChevronUp, X } from "@lucide/svelte";
  import { tr } from "../i18n";
  import MarkdownViewer from "./MarkdownViewer.svelte";
  import {
    askCompletion,
    dismissAskCompletionNotice,
    toggleAskCompletionExpanded,
  } from "../stores/chat.svelte";

  interface Props {
    repoPath: string | null;
    onOpenAsk: (sessionId: string) => void;
  }

  let { repoPath, onOpenAsk }: Props = $props();

  const notice = $derived(askCompletion.notice);

  function handleOpen() {
    if (!notice) return;
    onOpenAsk(notice.sessionId);
    dismissAskCompletionNotice();
  }

  function handleHeaderClick() {
    if (notice && !notice.expanded) {
      toggleAskCompletionExpanded();
    }
  }
</script>

{#if notice}
  <div
    class="tr-toast-in pointer-events-auto fixed bottom-4 right-4 z-[100] w-[min(420px,calc(100vw-2rem))] overflow-hidden rounded-2xl border border-tr-border-strong bg-tr-surface shadow-2xl"
    role="alertdialog"
    aria-labelledby="ask-notice-title"
  >
    <div class="flex items-start gap-2 border-b border-tr-border-strong px-4 py-3">
      <button
        type="button"
        class="min-w-0 flex-1 cursor-pointer text-left transition-opacity hover:opacity-90"
        onclick={handleHeaderClick}
      >
        <p id="ask-notice-title" class="text-sm font-semibold text-tr-ink">
          {tr("ask.notice.title")}
        </p>
        <p class="truncate text-xs text-tr-ink-3">{notice.title}</p>
      </button>
      <button
        type="button"
        class="tr-press shrink-0 rounded-lg p-1 text-tr-ink-3 transition-colors hover:bg-tr-elevated"
        aria-label={tr("ask.notice.close")}
        onclick={() => dismissAskCompletionNotice()}
      >
        <X size={16} strokeWidth={2} />
      </button>
    </div>

    {#if notice.expanded}
      <div class="max-h-64 overflow-y-auto px-4 py-3">
        <MarkdownViewer
          body={notice.answerMarkdown}
          {repoPath}
          compact
          allowMermaid={false}
        />
      </div>
    {/if}

    <div class="flex items-center gap-2 border-t border-tr-border-strong px-3 py-2">
      <button
        type="button"
        class="tr-press inline-flex items-center gap-1 rounded-lg px-2.5 py-1.5 text-xs text-tr-ink-2 transition-colors hover:bg-tr-elevated"
        onclick={() => toggleAskCompletionExpanded()}
      >
        {#if notice.expanded}
          <ChevronDown size={14} strokeWidth={2} />
          {tr("common.collapse")}
        {:else}
          <ChevronUp size={14} strokeWidth={2} />
          {tr("ask.notice.expand")}
        {/if}
      </button>
      <button
        type="button"
        class="tr-press ml-auto rounded-lg bg-tr-accent px-3 py-1.5 text-xs font-medium text-white transition-colors hover:bg-tr-accent-hover"
        onclick={handleOpen}
      >
        {tr("ask.notice.open")}
      </button>
    </div>
  </div>
{/if}
