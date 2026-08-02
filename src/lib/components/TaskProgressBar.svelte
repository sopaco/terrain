<script lang="ts">
  interface Props {
    projectSlug: string;
    stage?: string | null;
    message: string;
    oncancel?: (() => void) | null;
  }

  let { projectSlug, stage = null, message, oncancel = null }: Props = $props();
</script>

<div class="shrink-0 overflow-visible border-b border-tr-watch/30 bg-gradient-to-r from-tr-watch-soft to-transparent px-4 py-3">
  <div class="flex w-full items-start gap-3">
    <div
      class="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-tr-watch/30 bg-tr-watch-soft"
      aria-hidden="true"
    >
      <span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-tr-watch border-t-transparent"></span>
    </div>

    <div class="min-w-0 flex-1 overflow-visible">
      <div class="flex w-[130%] max-w-[calc(100vw-3rem)] flex-wrap items-center gap-2">
        <span class="text-sm font-medium text-tr-ink">正在生成知识资产</span>
        <span class="rounded-full bg-tr-page px-2 py-0.5 text-[10px] uppercase tracking-wide text-tr-watch">
          {projectSlug}
        </span>
        {#if stage}
          <span class="rounded-full border border-tr-watch/30 px-2 py-0.5 text-[10px] text-tr-watch">
            {stage}
          </span>
        {/if}
      </div>
      <p class="mt-1 w-[130%] max-w-[calc(100vw-3rem)] break-words text-xs leading-relaxed text-tr-watch">
        {message}
      </p>
      <div class="mt-2 h-1.5 w-[130%] max-w-[calc(100vw-3rem)] overflow-hidden rounded-full bg-tr-elevated">
        <div class="h-full w-[43%] animate-pulse rounded-full bg-tr-watch"></div>
      </div>
    </div>

    {#if oncancel}
      <button
        type="button"
        class="tr-press shrink-0 rounded-lg border border-tr-border-strong px-3 py-1.5 text-xs text-tr-ink-2 transition-colors hover:bg-tr-elevated"
        onclick={oncancel}
      >
        关闭
      </button>
    {/if}
  </div>
</div>
