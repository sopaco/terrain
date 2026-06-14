<script lang="ts">
  interface Props {
    projectSlug: string;
    stage?: string | null;
    message: string;
    oncancel?: (() => void) | null;
  }

  let { projectSlug, stage = null, message, oncancel = null }: Props = $props();
</script>

<div class="shrink-0 overflow-visible border-b border-amber-500/25 bg-gradient-to-r from-amber-500/10 to-orange-500/5 px-4 py-3">
  <div class="flex w-full items-start gap-3">
    <div
      class="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-amber-500/30 bg-amber-500/15"
      aria-hidden="true"
    >
      <span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-amber-200 border-t-transparent"></span>
    </div>

    <div class="min-w-0 flex-1 overflow-visible">
      <div class="flex w-[130%] max-w-[calc(100vw-3rem)] flex-wrap items-center gap-2">
        <span class="text-sm font-medium text-amber-50">正在生成知识资产</span>
        <span class="rounded-full bg-black/20 px-2 py-0.5 text-[10px] uppercase tracking-wide text-amber-200/80">
          {projectSlug}
        </span>
        {#if stage}
          <span class="rounded-full border border-amber-500/25 px-2 py-0.5 text-[10px] text-amber-100/90">
            {stage}
          </span>
        {/if}
      </div>
      <p class="mt-1 w-[130%] max-w-[calc(100vw-3rem)] break-words text-xs leading-relaxed text-amber-100/80">
        {message}
      </p>
      <div class="mt-2 h-1.5 w-[130%] max-w-[calc(100vw-3rem)] overflow-hidden rounded-full bg-amber-950/40">
        <div class="h-full w-[43%] animate-pulse rounded-full bg-amber-400/80"></div>
      </div>
    </div>

    {#if oncancel}
      <button
        type="button"
        class="shrink-0 rounded-lg border border-white/10 px-3 py-1.5 text-xs text-white/70 hover:bg-white/5"
        onclick={oncancel}
      >
        关闭
      </button>
    {/if}
  </div>
</div>
