<script lang="ts">
  export type UsageBarPoint = {
    label: string;
    tokens: number;
    cost: number;
  };

  export type UsageChartGranularity = "day" | "month" | "year";

  interface Props {
    bars: UsageBarPoint[];
    granularity?: UsageChartGranularity;
    metric?: "tokens" | "cost";
    emptyLabel?: string;
  }

  let {
    bars,
    granularity = "day",
    metric = "tokens",
    emptyLabel = "暂无数据",
  }: Props = $props();

  let hoveredIndex = $state<number | null>(null);

  const PLOT_HEIGHT = 132;
  const LABEL_AREA = $derived(granularity === "month" ? 40 : 24);

  const barLayout = $derived.by(() => {
    if (granularity === "month") {
      return { width: 34, gap: 12 };
    }
    if (granularity === "year") {
      return { width: 48, gap: 16 };
    }
    return { width: 28, gap: 6 };
  });

  const values = $derived(bars.map((b) => (metric === "cost" ? b.cost : b.tokens)));
  const maxValue = $derived(Math.max(...values, metric === "cost" ? 0.01 : 1));

  const chartMinWidth = $derived(
    Math.max(bars.length * (barLayout.width + barLayout.gap), 280),
  );

  const allLabels = $derived(bars.map((b) => b.label));

  const hoveredBar = $derived(hoveredIndex === null ? null : (bars[hoveredIndex] ?? null));

  function barHeight(value: number): number {
    if (value <= 0) return 0;
    return Math.max(4, (value / maxValue) * (PLOT_HEIGHT - 8));
  }

  function formatTokens(value: number): string {
    return value.toLocaleString();
  }

  function formatCost(value: number): string {
    if (value <= 0) return "$0.00";
    if (value < 0.01) return "< $0.01";
    return `$${value.toFixed(2)}`;
  }

  function axisLabel(label: string): string {
    if (granularity === "year" && /^\d{4}$/.test(label)) {
      return label;
    }
    if (granularity === "day" && /^\d{4}-\d{2}-\d{2}$/.test(label)) {
      return label.slice(5);
    }
    if (granularity === "month" && /^\d{4}-\d{2}$/.test(label)) {
      const month = Number.parseInt(label.slice(5), 10);
      const years = new Set(allLabels.map((l) => l.slice(0, 4)));
      if (years.size <= 1) return `${month}月`;
      const yy = label.slice(2, 4);
      return `${yy}/${month}`;
    }
    return label.length > 8 ? `${label.slice(0, 7)}…` : label;
  }

  function fullLabel(label: string): string {
    if (/^\d{4}-\d{2}$/.test(label)) {
      const year = label.slice(0, 4);
      const month = Number.parseInt(label.slice(5), 10);
      return `${year}年${month}月`;
    }
    if (/^\d{4}-\d{2}-\d{2}$/.test(label)) {
      const [year, month, day] = label.split("-");
      return `${year}年${Number.parseInt(month, 10)}月${Number.parseInt(day, 10)}日`;
    }
    if (/^\d{4}$/.test(label)) {
      return `${label}年`;
    }
    return label;
  }

  const useRotatedLabels = $derived(granularity === "month" && bars.length > 6);

  function selectBar(index: number) {
    hoveredIndex = index;
  }

  function clearBar() {
    hoveredIndex = null;
  }
</script>

{#if bars.length === 0}
  <div class="flex h-40 items-center justify-center rounded-xl border border-white/10 bg-white/[0.02] text-sm text-white/40">
    {emptyLabel}
  </div>
{:else}
  <div class="rounded-xl border border-white/10 bg-white/[0.02] px-3 pb-3 pt-3">
    <div class="overflow-x-auto">
      <div
        class="flex items-end"
        style:min-width="{chartMinWidth}px"
        style:min-height="{PLOT_HEIGHT + LABEL_AREA}px"
        role="list"
        aria-label="用量柱状图"
        onmouseleave={clearBar}
      >
        {#each bars as bar, i}
          {@const value = metric === "cost" ? bar.cost : bar.tokens}
          {@const h = barHeight(value)}
          {@const active = hoveredIndex === i}
          <button
            type="button"
            class="group flex shrink-0 flex-col items-center border-0 bg-transparent p-0 text-left"
            style:width="{barLayout.width}px"
            style:margin-right="{i < bars.length - 1 ? `${barLayout.gap}px` : '0'}"
            aria-label="{fullLabel(bar.label)} · {formatTokens(bar.tokens)} tokens · {formatCost(bar.cost)}"
            aria-pressed={active}
            onmouseenter={() => selectBar(i)}
            onfocus={() => selectBar(i)}
            onblur={clearBar}
          >
            <div
              class="flex w-full items-end"
              style:height="{PLOT_HEIGHT}px"
            >
              <div
                class="w-full rounded-t transition-colors {active
                  ? 'bg-indigo-400'
                  : 'bg-indigo-500/80 group-hover:bg-indigo-400'}"
                style:height="{h}px"
              ></div>
            </div>
            <span
              class="mt-1 block w-full truncate text-center text-[9px] leading-tight text-white/50 transition-colors {active
                ? 'text-white/80'
                : 'group-hover:text-white/70'} {useRotatedLabels ? '-rotate-[42deg] origin-top' : ''}"
              title={fullLabel(bar.label)}
            >
              {axisLabel(bar.label)}
            </span>
          </button>
        {/each}
      </div>
    </div>

    <div
      class="mt-3 rounded-lg border px-3 py-2.5 text-xs transition-colors {hoveredBar
        ? 'border-indigo-500/30 bg-indigo-500/[0.08]'
        : 'border-white/10 bg-black/20'}"
      aria-live="polite"
    >
      {#if hoveredBar}
        <p class="font-medium text-white/90">{fullLabel(hoveredBar.label)}</p>
        <div class="mt-1.5 flex flex-wrap gap-x-4 gap-y-1 text-white/70">
          <span>
            <span class="text-white/40">Tokens</span>
            {formatTokens(hoveredBar.tokens)}
          </span>
          <span>
            <span class="text-white/40">成本</span>
            {formatCost(hoveredBar.cost)}
          </span>
          <span>
            <span class="text-white/40">当前指标</span>
            {metric === "cost" ? formatCost(hoveredBar.cost) : formatTokens(hoveredBar.tokens)}
          </span>
        </div>
      {:else}
        <p class="text-white/40">将鼠标移到柱体上查看该时段用量与成本</p>
      {/if}
    </div>
  </div>
{/if}
