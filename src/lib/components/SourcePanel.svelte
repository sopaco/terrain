<script lang="ts">
  import MarkdownViewer from "./MarkdownViewer.svelte";
  import SourceCodeViewer from "./SourceCodeViewer.svelte";
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
  <div class="flex items-center justify-between border-b border-white/10 px-3 py-2">
    <span class="text-xs font-medium uppercase tracking-wide text-white/40">
      {slice.format === "markdown" ? "Document" : "Source"}
    </span>
    <button
      type="button"
      class="text-xs text-white/40 hover:text-white/70"
      onclick={onclose}
    >
      关闭
    </button>
  </div>
  <div class="border-b border-white/10 px-3 py-2 text-xs text-white/50">
    <div class="truncate font-mono" title={slice.file_path}>{slice.file_path}</div>
    {#if slice.format !== "markdown"}
      {#if slice.focus_line}
        <div class="mt-1">
          第 {slice.start_line}–{slice.end_line} 行 · 引用行 {slice.focus_line}
        </div>
      {:else if slice.start_line > 0 && slice.end_line > 0}
        <div class="mt-1">第 {slice.start_line}–{slice.end_line} 行</div>
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
          class="h-5 w-5 shrink-0 animate-spin rounded-full border-2 border-indigo-400/30 border-t-indigo-300"
        ></div>
        <p class="text-sm text-white/60">正在加载…</p>
      </div>
      <div class="space-y-2">
        {#each Array.from({ length: 8 }, (_, i) => i) as i (i)}
          <div
            class="h-3 animate-pulse rounded bg-white/[0.06]"
            style:width="{55 + (i % 4) * 10}%"
          ></div>
        {/each}
      </div>
    </div>
  {:else if errored}
    <div class="flex-1 overflow-y-auto p-4 text-sm text-red-300/90">
      <p class="font-medium">无法加载源文件</p>
      <p class="mt-2 whitespace-pre-wrap text-white/50">{slice.content}</p>
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
    <div class="min-h-0 flex-1 overflow-y-auto">
      <SourceCodeViewer
        content={slice.content}
        filePath={slice.file_path}
        startLine={slice.start_line}
        focusLine={slice.focus_line}
      />
    </div>
  {/if}
</div>
