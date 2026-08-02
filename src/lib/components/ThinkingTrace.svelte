<script lang="ts">
  import { THINKING_TRACE_LABELS } from "../terminology";
  import MarkdownViewer from "./MarkdownViewer.svelte";
  import ChevronIcon from "./icons/ChevronIcon.svelte";

  interface Props {
    content: string;
    active?: boolean;
    repoPath?: string | null;
  }

  let { content, active = false, repoPath = null }: Props = $props();

  let panelOpen = $state(false);

  const label = $derived(active ? THINKING_TRACE_LABELS.active : THINKING_TRACE_LABELS.done);
</script>

{#if content.trim()}
  <div class="my-2 rounded-lg border border-tr-border-strong bg-tr-page">
    <button
      type="button"
      class="flex w-full items-center gap-2 px-3 py-2 text-left text-xs transition-colors hover:bg-tr-elevated"
      onclick={() => (panelOpen = !panelOpen)}
      aria-expanded={panelOpen}
    >
      <ChevronIcon direction={panelOpen ? "down" : "right"} size={12} class="shrink-0 text-tr-ink-3" />
      <span class="font-medium text-tr-ink-2">{label}</span>
      {#if active}
        <span
          class="ml-auto inline-block h-3 w-3 animate-spin rounded-full border-2 border-tr-accent border-t-transparent"
          aria-hidden="true"
        ></span>
      {/if}
    </button>

    {#if panelOpen}
      <div class="border-t border-tr-border px-3 py-2">
        <MarkdownViewer body={content} {repoPath} compact allowMermaid={false} highlight={false} />
      </div>
    {/if}
  </div>
{/if}
