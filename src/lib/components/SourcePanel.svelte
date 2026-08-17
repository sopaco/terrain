<script lang="ts">
  import MarkdownViewer from "./MarkdownViewer.svelte";
  import SourceCodeViewer from "./SourceCodeViewer.svelte";
  import { tr } from "../i18n";
  import type { SourceCitation, SourceSlice } from "../types";

  interface Props {
    slice: SourceSlice;
    repoPath?: string | null;
    onclose: () => void;
    onSourceClick?: (citation: SourceCitation) => void;
  }

  let { slice, repoPath = null, onclose, onSourceClick }: Props = $props();

  const loading = $derived(slice.status === "loading");
  const errored = $derived(slice.status === "error");
</script>

<div
  class="flex min-h-0 flex-1 flex-col"
  role="complementary"
  aria-label="Source"
>
  <div class="flex items-center justify-between border-b border-tr-border-strong px-3 py-2">
    <span class="text-xs font-medium uppercase tracking-wide text-tr-ink-3">
      {slice.format === "markdown" ? "Document" : "Source"}
    </span>
    <button
      type="button"
      class="text-xs text-tr-ink-3 transition-colors hover:text-tr-ink-2"
      onclick={onclose}
    >
      {tr("common.close")}
    </button>
  </div>
  <div class="border-b border-tr-border-strong px-3 py-2 text-xs text-tr-ink-3">
    <div class="truncate font-mono" title={slice.file_path}>{slice.file_path}</div>
    {#if slice.format !== "markdown"}
      {#if slice.focus_line}
        <div class="mt-1">
          {tr("ask.source.linesWithFocus", { start: slice.start_line, end: slice.end_line, focus: slice.focus_line })}
        </div>
      {:else if slice.start_line > 0 && slice.end_line > 0}
        <div class="mt-1">{tr("ask.source.lines", { start: slice.start_line, end: slice.end_line })}</div>
      {/if}
    {/if}
  </div>
  {#if loading}
    <div
      class="flex flex-1 flex-col gap-3 p-4"
      role="status"
      aria-live="polite"
      aria-busy="true"
    >
      <div class="flex items-center gap-3">
        <div
          class="h-5 w-5 shrink-0 animate-spin rounded-full border-2 border-tr-accent-soft-strong border-t-tr-accent"
        ></div>
        <p class="text-sm text-tr-ink-2">{tr("ask.source.loading")}</p>
      </div>
      <div class="space-y-2">
        {#each Array.from({ length: 8 }, (_, i) => i) as i (i)}
          <div
            class="h-3 animate-pulse rounded bg-tr-raised"
            style:width="{55 + (i % 4) * 10}%"
          ></div>
        {/each}
      </div>
    </div>
  {:else if errored}
    <div class="flex-1 overflow-y-auto p-4 text-sm text-tr-critical">
      <p class="font-medium">{tr("ask.source.loadFailed")}</p>
      <p class="mt-2 whitespace-pre-wrap text-tr-ink-3">{slice.content}</p>
    </div>
  {:else if slice.format === "markdown"}
    <div class="flex-1 overflow-y-auto p-4">
      <MarkdownViewer
        body={slice.content}
        repoPath={slice.repo_path || repoPath}
        compact
        {onSourceClick}
      />
    </div>
  {:else}
    <div class="flex min-h-0 flex-1 flex-col">
      <SourceCodeViewer
        content={slice.content}
        filePath={slice.file_path}
        startLine={slice.start_line || 1}
        focusLine={slice.focus_line}
      />
    </div>
  {/if}
</div>
