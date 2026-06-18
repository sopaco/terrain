<script lang="ts">
  export interface OverviewActionItem {
    id: string;
    priority: number;
    accent: "rose" | "amber" | "violet";
    title: string;
    detail: string;
    hint?: string;
    actionLabel: string;
    busyLabel?: string;
    onAction?: () => void;
    disabled?: boolean;
    busy?: boolean;
  }

  interface Props {
    items: OverviewActionItem[];
    progressNote?: string | null;
  }

  let { items, progressNote = null }: Props = $props();

  let moreOpen = $state(false);

  const sorted = $derived([...items].sort((a, b) => a.priority - b.priority));
  const primary = $derived(sorted[0] ?? null);
  const secondary = $derived(sorted.slice(1));

  const accentBorder: Record<OverviewActionItem["accent"], string> = {
    rose: "border-l-rose-500",
    amber: "border-l-amber-500",
    violet: "border-l-violet-500",
  };

  const accentButton: Record<OverviewActionItem["accent"], string> = {
    rose: "bg-rose-600 hover:bg-rose-500",
    amber: "bg-amber-600 hover:bg-amber-500",
    violet: "bg-violet-600 hover:bg-violet-500",
  };
</script>

{#snippet actionRow(item: OverviewActionItem, compact = false)}
  <div
    class={`rounded-xl border border-white/8 bg-[#14171c] border-l-[3px] ${accentBorder[item.accent]} ${
      compact ? "px-4 py-3" : "px-5 py-4"
    }`}
  >
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div class="min-w-0">
        <p class="text-sm font-medium text-white/90">{item.title}</p>
        <p class="mt-1 text-xs leading-relaxed text-white/50">{item.detail}</p>
        {#if item.hint}
          <p class="mt-2 text-[11px] text-white/40">{item.hint}</p>
        {/if}
        {#if progressNote}
          <p class="mt-2 text-xs text-indigo-200/80">{progressNote}</p>
        {/if}
      </div>
      {#if item.onAction}
        <button
          type="button"
          class={`shrink-0 rounded-xl px-4 py-2 text-sm font-medium text-white disabled:opacity-50 ${accentButton[item.accent]}`}
          disabled={item.disabled || item.busy}
          onclick={item.onAction}
        >
          {item.busy ? (item.busyLabel ?? "处理中…") : item.actionLabel}
        </button>
      {/if}
    </div>
  </div>
{/snippet}

{#if primary}
  <div class="space-y-2">
    {@render actionRow(primary)}

    {#if secondary.length > 0}
      <button
        type="button"
        class="flex w-full items-center gap-2 rounded-lg px-1 py-1.5 text-left text-xs text-white/45 transition-colors hover:text-white/65"
        aria-expanded={moreOpen}
        onclick={() => (moreOpen = !moreOpen)}
      >
        <span class="text-white/30">{moreOpen ? "▾" : "▸"}</span>
        还有 {secondary.length} 项待处理
      </button>

      {#if moreOpen}
        <div class="space-y-2">
          {#each secondary as item (item.id)}
            {@render actionRow(item, true)}
          {/each}
        </div>
      {/if}
    {/if}
  </div>
{/if}
