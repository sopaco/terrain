<script lang="ts">
  import { ChartColumn } from "@lucide/svelte";
  import {
    usageBadgePeriodLabel,
    usageDisplay,
  } from "../stores/usageDisplay.svelte";
  import type { UsageSnapshot, UsageTotals } from "../types";

  interface Props {
    snapshot: UsageSnapshot | null;
    loading: boolean;
    onclick: () => void;
  }

  let { snapshot, loading, onclick }: Props = $props();

  const period = $derived(usageDisplay.badgePeriod);

  function formatTokens(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
    return n.toLocaleString();
  }

  function formatCost(usd: number): string {
    if (usd <= 0) return "$0";
    if (usd < 0.01) return "<$0.01";
    return `$${usd.toFixed(2)}`;
  }

  const activeTotals = $derived.by((): UsageTotals | null => {
    if (!snapshot) return null;
    return period === "month" ? snapshot.month : snapshot.today;
  });

  const periodLabel = $derived(usageBadgePeriodLabel(period));

  const hasData = $derived(
    !!activeTotals && (activeTotals.total_tokens > 0 || activeTotals.total_cost_usd > 0),
  );
  const noSources = $derived(
    !!snapshot && snapshot.probe.sources.every((s) => !s.detected),
  );
  const unavailable = $derived(
    !!snapshot && !snapshot.probe.ccusage_available,
  );
</script>

<button
  type="button"
  class="hidden shrink-0 items-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-xs transition-colors hover:bg-white/5 sm:inline-flex
    {loading ? 'border-white/10 text-white/40' : hasData ? 'border-indigo-500/35 bg-indigo-500/[0.06] text-indigo-100/90' : 'border-white/10 text-white/50'}"
  title="开发者 Token 用量（{periodLabel}）"
  aria-label="开发者 Token 用量，{periodLabel}"
  disabled={loading && !snapshot}
  onclick={onclick}
>
  <ChartColumn
    size={14}
    strokeWidth={2}
    class="shrink-0 {hasData ? 'text-indigo-300/90' : 'text-white/45'}"
    aria-hidden="true"
  />
  {#if loading && !snapshot}
    <span>用量…</span>
  {:else if unavailable}
    <span>用量 —</span>
  {:else if noSources}
    <span>用量 —</span>
  {:else if hasData && activeTotals}
    <span>{periodLabel} {formatCost(activeTotals.total_cost_usd)} · {formatTokens(activeTotals.total_tokens)}</span>
  {:else}
    <span>{periodLabel} $0</span>
  {/if}
</button>
