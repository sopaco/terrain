<script lang="ts">
  import { ChartColumn } from "@lucide/svelte";
  import { getUsageSnapshot } from "../api";
  import {
    setUsageBadgePeriod,
    usageDisplay,
    type UsageBadgePeriod,
  } from "../stores/usageDisplay.svelte";
  import type { UsagePeriodEntry, UsageSnapshot, UsageTotals } from "../types";
  import SlideDrawer from "./SlideDrawer.svelte";

  interface Props {
    open: boolean;
    onclose: () => void;
    initialSnapshot?: UsageSnapshot | null;
  }

  let { open, onclose, initialSnapshot = null }: Props = $props();

  let snapshot = $state<UsageSnapshot | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let tab = $state<"daily" | "sessions">("daily");

  $effect(() => {
    if (open) {
      if (initialSnapshot) snapshot = initialSnapshot;
      void refresh(true);
    }
  });

  async function refresh(force: boolean) {
    loading = true;
    error = null;
    try {
      snapshot = await getUsageSnapshot("full", force);
      if (snapshot.error) {
        error = snapshot.error;
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function formatTokens(n: number): string {
    return n.toLocaleString();
  }

  function formatCost(usd: number): string {
    if (usd <= 0) return "$0.00";
    return `$${usd.toFixed(2)}`;
  }

  function formatTime(ms: number): string {
    if (!ms) return "";
    return new Date(ms).toLocaleString();
  }

  function totalsCard(label: string, totals: UsageTotals) {
    return { label, totals };
  }

  const summaryCards = $derived(
    snapshot
      ? [
          totalsCard("今日", snapshot.today),
          totalsCard("近 7 天", snapshot.week),
          totalsCard("本月", snapshot.month),
        ]
      : [],
  );

  const dailyRows = $derived(
    [...(snapshot?.daily ?? [])].sort((a, b) => b.period.localeCompare(a.period)),
  );

  const sessionRows = $derived(
    [...(snapshot?.sessions ?? [])].sort((a, b) => b.total_tokens - a.total_tokens),
  );

  const activeRows = $derived(tab === "daily" ? dailyRows : sessionRows);

  function rowLabel(row: UsagePeriodEntry): string {
    return tab === "daily" ? row.period : row.period || row.agent || "会话";
  }

  function agentsLabel(row: UsagePeriodEntry): string {
    if (row.agents.length > 0) return row.agents.join(", ");
    return row.agent ?? "—";
  }

  const badgePeriod = $derived(usageDisplay.badgePeriod);

  function selectBadgePeriod(period: UsageBadgePeriod) {
    setUsageBadgePeriod(period);
  }
</script>

<SlideDrawer {open} {onclose} ariaLabel="开发者 Token 用量" widthClass="w-[min(92vw,44rem)]">
  <header class="flex shrink-0 flex-col gap-3 border-b border-white/10 px-5 py-4 sm:flex-row sm:items-center sm:justify-between">
    <div class="flex min-w-0 items-start gap-2.5">
      <div class="mt-0.5 rounded-lg border border-indigo-500/25 bg-indigo-500/10 p-1.5 text-indigo-300/90">
        <ChartColumn size={16} strokeWidth={2} aria-hidden="true" />
      </div>
      <div class="min-w-0">
        <h2 class="text-base font-semibold text-white">开发者 Token 用量</h2>
        <p class="mt-0.5 text-xs text-white/45">
          基于 ccusage 读取本地 Agent 日志，数据不上传
        </p>
      </div>
    </div>
    <div class="flex flex-wrap items-center gap-2 sm:justify-end">
      <div
        class="flex items-center gap-1.5 rounded-lg border border-white/10 bg-white/[0.03] px-2 py-1"
        role="group"
        aria-label="顶栏显示维度"
      >
        <span class="text-[10px] text-white/40">顶栏</span>
        <button
          type="button"
          class="rounded-md px-2 py-0.5 text-xs transition-colors {badgePeriod === 'day' ? 'bg-indigo-600 text-white' : 'text-white/60 hover:bg-white/5'}"
          aria-pressed={badgePeriod === "day"}
          onclick={() => selectBadgePeriod("day")}
        >
          今日
        </button>
        <button
          type="button"
          class="rounded-md px-2 py-0.5 text-xs transition-colors {badgePeriod === 'month' ? 'bg-indigo-600 text-white' : 'text-white/60 hover:bg-white/5'}"
          aria-pressed={badgePeriod === "month"}
          onclick={() => selectBadgePeriod("month")}
        >
          本月
        </button>
      </div>
      <button
        type="button"
        class="rounded-lg border border-white/10 px-2.5 py-1.5 text-xs text-white/70 hover:bg-white/5 disabled:opacity-40"
        disabled={loading}
        onclick={() => void refresh(true)}
      >
        {loading ? "刷新中…" : "刷新"}
      </button>
      <button
        type="button"
        class="rounded-lg border border-white/10 px-2.5 py-1.5 text-xs text-white/70 hover:bg-white/5"
        aria-label="关闭"
        onclick={onclose}
      >
        ✕
      </button>
    </div>
  </header>

  <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">
    {#if loading && !snapshot}
      <div class="flex items-center gap-2 py-8 text-sm text-white/50">
        <span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-indigo-300 border-t-transparent"></span>
        正在读取本地用量日志…
      </div>
    {:else if snapshot}
      {#if error}
        <div class="mb-4 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-sm text-amber-100/90">
          {error}
        </div>
      {/if}

      <div class="mb-5 grid gap-3 sm:grid-cols-3">
        {#each summaryCards as card}
          {@const highlighted =
            (badgePeriod === "day" && card.label === "今日") ||
            (badgePeriod === "month" && card.label === "本月")}
          <div
            class="rounded-xl border p-3 {highlighted
              ? 'border-indigo-500/35 bg-indigo-500/[0.08]'
              : 'border-white/10 bg-white/[0.03]'}"
          >
            <p class="text-[10px] uppercase tracking-wide text-white/40">
              {card.label}
              {#if highlighted}
                <span class="ml-1 normal-case text-indigo-300/80">· 顶栏</span>
              {/if}
            </p>
            <p class="mt-1 text-lg font-semibold text-white">{formatCost(card.totals.total_cost_usd)}</p>
            <p class="mt-0.5 text-xs text-white/45">{formatTokens(card.totals.total_tokens)} tokens</p>
          </div>
        {/each}
      </div>

      <section class="mb-5">
        <h3 class="mb-2 text-xs font-medium uppercase tracking-wide text-white/40">数据来源</h3>
        <ul class="space-y-2">
          {#each snapshot.probe.sources as source}
            <li class="flex items-start justify-between gap-3 rounded-lg border border-white/10 bg-black/20 px-3 py-2 text-xs">
              <div class="min-w-0">
                <div class="flex items-center gap-2">
                  <span
                    class="inline-block h-1.5 w-1.5 rounded-full {source.detected ? 'bg-emerald-400' : 'bg-white/20'}"
                  ></span>
                  <span class="font-medium text-white/85">{source.label}</span>
                </div>
                {#if source.path}
                  <p class="mt-1 truncate text-white/40" title={source.path}>{source.path}</p>
                {/if}
              </div>
              <span class="shrink-0 text-white/45">
                {source.detected ? `${source.file_count} 个文件` : "未检测"}
              </span>
            </li>
          {/each}
        </ul>
        {#if snapshot.probe.ccusage_version}
          <p class="mt-2 text-[10px] text-white/30">
            ccusage {snapshot.probe.ccusage_version}
            {#if snapshot.cached}· 缓存{/if}
            {#if snapshot.generated_at}· 更新于 {formatTime(snapshot.generated_at)}{/if}
          </p>
        {/if}
      </section>

      <div class="mb-3 flex gap-1 rounded-lg border border-white/10 bg-white/[0.03] p-0.5">
        <button
          type="button"
          class="flex-1 rounded-md px-3 py-1.5 text-xs transition-colors {tab === 'daily' ? 'bg-indigo-600 text-white' : 'text-white/60 hover:bg-white/5'}"
          onclick={() => (tab = "daily")}
        >
          按日
        </button>
        <button
          type="button"
          class="flex-1 rounded-md px-3 py-1.5 text-xs transition-colors {tab === 'sessions' ? 'bg-indigo-600 text-white' : 'text-white/60 hover:bg-white/5'}"
          onclick={() => (tab = "sessions")}
        >
          按会话
        </button>
      </div>

      {#if activeRows.length === 0}
        <p class="py-6 text-center text-sm text-white/45">
          {tab === "sessions" ? "暂无会话数据" : "所选时间范围内暂无用量记录"}
        </p>
      {:else}
        <div class="overflow-x-auto rounded-xl border border-white/10">
          <table class="w-full min-w-[36rem] text-left text-xs">
            <thead class="border-b border-white/10 bg-white/[0.03] text-white/45">
              <tr>
                <th class="px-3 py-2 font-medium">{tab === "daily" ? "日期" : "会话"}</th>
                <th class="px-3 py-2 font-medium">Agent</th>
                <th class="px-3 py-2 font-medium text-right">输入</th>
                <th class="px-3 py-2 font-medium text-right">输出</th>
                <th class="px-3 py-2 font-medium text-right">Cache</th>
                <th class="px-3 py-2 font-medium text-right">成本</th>
              </tr>
            </thead>
            <tbody>
              {#each activeRows as row}
                <tr class="border-b border-white/5 last:border-0 hover:bg-white/[0.02]">
                  <td class="px-3 py-2 text-white/80">{rowLabel(row)}</td>
                  <td class="px-3 py-2 text-white/55">{agentsLabel(row)}</td>
                  <td class="px-3 py-2 text-right text-white/70">{formatTokens(row.input_tokens)}</td>
                  <td class="px-3 py-2 text-right text-white/70">{formatTokens(row.output_tokens)}</td>
                  <td class="px-3 py-2 text-right text-white/50">
                    {formatTokens(row.cache_creation_tokens + row.cache_read_tokens)}
                  </td>
                  <td class="px-3 py-2 text-right text-indigo-200/90">{formatCost(row.total_cost_usd)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}

      <p class="mt-4 text-[10px] leading-relaxed text-white/30">
        成本为基于 LiteLLM 定价的估算值，可能与实际账单有偏差。Cursor IDE 用量不在 ccusage 支持范围内。
      </p>
    {/if}
  </div>
</SlideDrawer>
