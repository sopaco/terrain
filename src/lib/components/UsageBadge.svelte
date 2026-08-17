<script lang="ts">
  import { ChartColumn } from "@lucide/svelte";
  import { tr } from "../i18n";
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
  class="tr-press hidden shrink-0 items-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-xs transition-colors hover:bg-tr-elevated sm:inline-flex
    {loading ? 'border-tr-border-strong text-tr-ink-3' : hasData ? 'border-tr-accent-soft-strong bg-tr-accent-soft text-tr-on-accent' : 'border-tr-border-strong text-tr-ink-3'}"
  title={tr("usage.badge.tooltip", { period: periodLabel })}
  aria-label={tr("usage.badge.ariaLabel", { period: periodLabel })}
  disabled={loading && !snapshot}
  onclick={onclick}
>
  <ChartColumn
    size={14}
    strokeWidth={2}
    class="shrink-0 {hasData ? 'text-tr-accent' : 'text-tr-ink-3'}"
    aria-hidden="true"
  />
  {#if loading && !snapshot}
    <span>{tr("usage.badge.loading")}</span>
  {:else if unavailable}
    <span>{tr("usage.badge.unavailable")}</span>
  {:else if noSources}
    <span>{tr("usage.badge.unavailable")}</span>
  {:else if hasData && activeTotals}
    <span class="flex items-center gap-1.5">
      <span class="text-tr-ink-3">{periodLabel}</span>
      <span class="flex items-center gap-1">
        <span class="text-tr-ink-3">{tr("usage.badge.cost")}</span>
        <span class="font-medium tabular-nums text-tr-ink">{formatCost(activeTotals.total_cost_usd)}</span>
      </span>
      <span class="text-tr-ink-4">·</span>
      <span class="flex items-center gap-1">
        <span class="text-tr-ink-3">Token</span>
        <span class="font-medium tabular-nums text-tr-ink">{formatTokens(activeTotals.total_tokens)}</span>
      </span>
    </span>
  {:else}
    <span>{tr("usage.badge.zero", { period: periodLabel })}</span>
  {/if}
</button>
