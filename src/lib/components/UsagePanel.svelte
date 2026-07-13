<script lang="ts">
    import { ChartColumn } from "@lucide/svelte";
    import { getUsageSnapshot } from "../api";
    import {
        setUsageBadgePeriod,
        usageDisplay,
        type UsageBadgePeriod,
    } from "../stores/usageDisplay.svelte";
    import type { UsagePeriodEntry, UsageSnapshot } from "../types";
    import UsageBarChart, { type UsageBarPoint } from "./UsageBarChart.svelte";
    import UsageDetailTable from "./UsageDetailTable.svelte";
    import CloseButton from "./icons/CloseButton.svelte";
    import SlideDrawer from "./SlideDrawer.svelte";

    type ChartPeriod = "day" | "month" | "year";

    interface Props {
        open: boolean;
        onclose: () => void;
        initialSnapshot?: UsageSnapshot | null;
    }

    let { open, onclose, initialSnapshot = null }: Props = $props();

    let snapshot = $state<UsageSnapshot | null>(null);
    let loading = $state(false);
    let error = $state<string | null>(null);
    let chartPeriod = $state<ChartPeriod>("day");
    let chartMetric = $state<"tokens" | "cost">("tokens");

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

    const badgePeriod = $derived(usageDisplay.badgePeriod);

    function selectBadgePeriod(period: UsageBadgePeriod) {
        setUsageBadgePeriod(period);
    }

    function sortByPeriodAsc(rows: UsagePeriodEntry[]): UsagePeriodEntry[] {
        return [...rows].sort((a, b) => a.period.localeCompare(b.period));
    }

    function sortByPeriodDesc(rows: UsagePeriodEntry[]): UsagePeriodEntry[] {
        return [...rows].sort((a, b) => b.period.localeCompare(a.period));
    }

    function sortByTokensDesc(rows: UsagePeriodEntry[]): UsagePeriodEntry[] {
        return [...rows].sort((a, b) => b.total_tokens - a.total_tokens);
    }

    function aggregateYearly(monthly: UsagePeriodEntry[]): UsageBarPoint[] {
        const byYear = new Map<string, UsageBarPoint>();
        for (const row of monthly) {
            const year = row.period.slice(0, 4);
            if (!/^\d{4}$/.test(year)) continue;
            const prev = byYear.get(year) ?? {
                label: year,
                tokens: 0,
                cost: 0,
            };
            byYear.set(year, {
                label: year,
                tokens: prev.tokens + row.total_tokens,
                cost: prev.cost + row.total_cost_usd,
            });
        }
        return [...byYear.values()]
            .sort((a, b) => a.label.localeCompare(b.label))
            .slice(-5);
    }

    function toBarPoints(rows: UsagePeriodEntry[]): UsageBarPoint[] {
        return rows.map((row) => ({
            label: row.period,
            tokens: row.total_tokens,
            cost: row.total_cost_usd,
        }));
    }

    const chartBars = $derived.by((): UsageBarPoint[] => {
        if (!snapshot) return [];
        if (chartPeriod === "day") {
            return toBarPoints(sortByPeriodAsc(snapshot.daily).slice(-30));
        }
        if (chartPeriod === "month") {
            return toBarPoints(sortByPeriodAsc(snapshot.monthly).slice(-12));
        }
        return aggregateYearly(snapshot.monthly);
    });

    const detailConfig = $derived.by(() => {
        if (chartPeriod === "day") {
            return {
                title: "会话明细",
                labelColumn: "会话",
                rows: sortByTokensDesc(snapshot?.sessions ?? []),
                rowLabel: sessionRowLabel,
                emptyLabel: "近 30 天暂无会话数据",
            };
        }
        if (chartPeriod === "month") {
            return {
                title: "按日明细",
                labelColumn: "日期",
                rows: sortByPeriodDesc(snapshot?.daily ?? []),
                rowLabel: (row: UsagePeriodEntry) => row.period,
                emptyLabel: "暂无按日用量记录",
            };
        }
        return {
            title: "按月明细",
            labelColumn: "月份",
            rows: sortByPeriodDesc(snapshot?.monthly ?? []),
            rowLabel: (row: UsagePeriodEntry) => row.period,
            emptyLabel: "暂无按月用量记录",
        };
    });

    function sessionRowLabel(row: UsagePeriodEntry): string {
        if (row.period.length >= 8) return `会话 ${row.period.slice(0, 8)}`;
        return row.period || "会话";
    }

    const summaryCards = $derived(
        snapshot
            ? [
                  { label: "今日", totals: snapshot.today },
                  { label: "近 7 天", totals: snapshot.week },
                  { label: "本月", totals: snapshot.month },
              ]
            : [],
    );

    const chartPeriodOptions: { id: ChartPeriod; label: string }[] = [
        { id: "day", label: "按日" },
        { id: "month", label: "按月" },
        { id: "year", label: "按年" },
    ];
</script>

<SlideDrawer
    {open}
    {onclose}
    ariaLabel="开发者 Token 用量"
    widthClass="w-[min(92vw,44rem)]"
>
    <header
        class="flex shrink-0 flex-col gap-3 border-b border-white/10 px-5 py-4 sm:flex-row sm:items-center sm:justify-between"
    >
        <div class="flex min-w-0 items-center gap-2.5">
            <div
                class="flex shrink-0 items-center justify-center rounded-lg border border-indigo-500/25 bg-indigo-500/10 p-1.5 text-indigo-300/90"
            >
                <ChartColumn size={16} strokeWidth={2} aria-hidden="true" />
            </div>
            <div class="min-w-0">
                <h2 class="text-base font-semibold text-white">
                    开发者 Token 用量
                </h2>
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
                    class="rounded-md px-2 py-0.5 text-xs transition-colors {badgePeriod ===
                    'day'
                        ? 'bg-indigo-600 text-white'
                        : 'text-white/60 hover:bg-white/5'}"
                    aria-pressed={badgePeriod === "day"}
                    onclick={() => selectBadgePeriod("day")}
                >
                    今日
                </button>
                <button
                    type="button"
                    class="rounded-md px-2 py-0.5 text-xs transition-colors {badgePeriod ===
                    'month'
                        ? 'bg-indigo-600 text-white'
                        : 'text-white/60 hover:bg-white/5'}"
                    aria-pressed={badgePeriod === "month"}
                    onclick={() => selectBadgePeriod("month")}
                >
                    本月
                </button>
            </div>
            <button
                type="button"
                class="rounded-lg border border-white/10 px-2.5 min-w-14 py-1.5 text-xs text-white/70 hover:bg-white/5 disabled:opacity-40"
                disabled={loading}
                onclick={() => void refresh(true)}
            >
                {loading ? "刷新中" : "刷新"}
            </button>
            <CloseButton onclick={onclose} class="px-2.5 py-1.5 text-xs" />
        </div>
    </header>

    <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">
        {#if loading && !snapshot}
            <div class="flex items-center gap-2 py-8 text-sm text-white/50">
                <span
                    class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-indigo-300 border-t-transparent"
                ></span>
                正在读取本地用量日志…
            </div>
        {:else if snapshot}
            {#if error}
                <div
                    class="mb-4 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-sm text-amber-100/90"
                >
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
                        <p
                            class="text-[10px] uppercase tracking-wide text-white/40"
                        >
                            {card.label}
                            {#if highlighted}
                                <span
                                    class="ml-1 normal-case text-indigo-300/80"
                                    >· 顶栏</span
                                >
                            {/if}
                        </p>
                        <p class="mt-1 text-lg font-semibold text-white">
                            {formatCost(card.totals.total_cost_usd)}
                        </p>
                        <p class="mt-0.5 text-xs text-white/45">
                            {formatTokens(card.totals.total_tokens)} tokens
                        </p>
                    </div>
                {/each}
            </div>

            <section class="mb-5">
                <h3
                    class="mb-2 text-xs font-medium uppercase tracking-wide text-white/40"
                >
                    数据来源
                </h3>
                <ul class="space-y-2">
                    {#each snapshot.probe.sources as source}
                        <li
                            class="flex items-center justify-between gap-3 rounded-lg border border-white/10 bg-black/20 px-3 py-2 text-xs"
                        >
                            <div class="flex min-w-0 items-center gap-2">
                                <span
                                    class="inline-block h-1.5 w-1.5 shrink-0 rounded-full {source.detected
                                        ? 'bg-emerald-400'
                                        : 'bg-white/20'}"
                                ></span>
                                <span class="font-medium text-white/85"
                                    >{source.label}</span
                                >
                            </div>
                            <span
                                class="shrink-0 {source.detected
                                    ? 'text-emerald-300/80'
                                    : 'text-white/40'}"
                            >
                                {source.detected ? "已安装" : "未检测到"}
                            </span>
                        </li>
                    {/each}
                </ul>
                {#if snapshot.generated_at}
                    <p class="mt-2 text-[10px] text-white/30">
                        {#if snapshot.cached}缓存数据 ·
                        {/if}更新于 {formatTime(snapshot.generated_at)}
                    </p>
                {/if}
            </section>

            <section class="mb-5">
                <div
                    class="mb-3 flex flex-wrap items-center justify-between gap-2"
                >
                    <div
                        class="flex gap-1 rounded-lg border border-white/10 bg-white/[0.03] p-0.5"
                    >
                        {#each chartPeriodOptions as option}
                            <button
                                type="button"
                                class="rounded-md px-3 py-1.5 text-xs transition-colors {chartPeriod ===
                                option.id
                                    ? 'bg-indigo-600 text-white'
                                    : 'text-white/60 hover:bg-white/5'}"
                                aria-pressed={chartPeriod === option.id}
                                onclick={() => (chartPeriod = option.id)}
                            >
                                {option.label}
                            </button>
                        {/each}
                    </div>
                    <div
                        class="flex gap-1 rounded-lg border border-white/10 bg-white/[0.03] p-0.5"
                    >
                        <button
                            type="button"
                            class="rounded-md px-2.5 py-1 text-xs transition-colors {chartMetric ===
                            'tokens'
                                ? 'bg-white/10 text-white'
                                : 'text-white/50 hover:bg-white/5'}"
                            aria-pressed={chartMetric === "tokens"}
                            onclick={() => (chartMetric = "tokens")}
                        >
                            Tokens
                        </button>
                        <button
                            type="button"
                            class="rounded-md px-2.5 py-1 text-xs transition-colors {chartMetric ===
                            'cost'
                                ? 'bg-white/10 text-white'
                                : 'text-white/50 hover:bg-white/5'}"
                            aria-pressed={chartMetric === "cost"}
                            onclick={() => (chartMetric = "cost")}
                        >
                            成本
                        </button>
                    </div>
                </div>
                <UsageBarChart
                    bars={chartBars}
                    granularity={chartPeriod}
                    metric={chartMetric}
                />
            </section>

            <UsageDetailTable
                title={detailConfig.title}
                labelColumn={detailConfig.labelColumn}
                rows={detailConfig.rows}
                rowLabel={detailConfig.rowLabel}
                emptyLabel={detailConfig.emptyLabel}
            />

            <p class="mt-4 text-[10px] leading-relaxed text-white/30">
                成本为估算值，可能与实际账单有偏差。Cursor IDE
                用量暂不支持统计。
            </p>
        {/if}
    </div>
</SlideDrawer>
