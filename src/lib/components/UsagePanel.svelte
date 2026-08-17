<script lang="ts">
    import { ChartColumn } from "@lucide/svelte";
    import { getUsageSnapshot, openLocalPath } from "../api";
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
    import { tr } from "../i18n";

    type ChartPeriod = "day" | "month" | "year";

    interface Props {
        open: boolean;
        onclose: () => void;
        initialSnapshot?: UsageSnapshot | null;
        /** 独立窗口模式：不使用 SlideDrawer，占满整窗 */
        standalone?: boolean;
    }

    let {
        open,
        onclose,
        initialSnapshot = null,
        standalone = false,
    }: Props = $props();

    let snapshot = $state<UsageSnapshot | null>(null);
    let loading = $state(false);
    let error = $state<string | null>(null);
    let chartPeriod = $state<ChartPeriod>("day");
    let chartMetric = $state<"tokens" | "cost">("tokens");

    $effect(() => {
        if (open || standalone) {
            if (initialSnapshot) snapshot = initialSnapshot;
            void refresh(false);
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
                title: tr("usage.detail.sessionTitle"),
                labelColumn: tr("usage.detail.sessionColumn"),
                rows: sortByTokensDesc(snapshot?.sessions ?? []),
                rowLabel: sessionRowLabel,
                emptyLabel: tr("usage.detail.emptySessions"),
            };
        }
        if (chartPeriod === "month") {
            return {
                title: tr("usage.detail.dailyTitle"),
                labelColumn: tr("usage.detail.dateColumn"),
                rows: sortByPeriodDesc(snapshot?.daily ?? []),
                rowLabel: (row: UsagePeriodEntry) => row.period,
                emptyLabel: tr("usage.detail.emptyDaily"),
            };
        }
        return {
            title: tr("usage.detail.monthlyTitle"),
            labelColumn: tr("usage.detail.monthColumn"),
            rows: sortByPeriodDesc(snapshot?.monthly ?? []),
            rowLabel: (row: UsagePeriodEntry) => row.period,
            emptyLabel: tr("usage.detail.emptyMonthly"),
        };
    });

    function sessionRowLabel(row: UsagePeriodEntry): string {
        if (row.period.length >= 8)
            return tr("usage.detail.sessionPrefix", {
                id: row.period.slice(0, 8),
            });
        return row.period || tr("usage.detail.sessionColumn");
    }

    function sessionOpenPath(row: UsagePeriodEntry): string | null {
        return row.source_path ?? null;
    }

    async function openSessionPath(path: string) {
        try {
            await openLocalPath(path);
        } catch (e) {
            error = tr("terms.msg.openPathFailed", { error: String(e) });
        }
    }

    const summaryCards = $derived(
        snapshot
            ? [
                  { label: tr("usage.period.today"), totals: snapshot.today },
                  { label: tr("usage.period.week"), totals: snapshot.week },
                  { label: tr("usage.period.month"), totals: snapshot.month },
              ]
            : [],
    );

    const detailLoading = $derived(
        loading &&
            chartPeriod === "day" &&
            (snapshot?.sessions.length ?? 0) === 0,
    );

    const chartPeriodOptions: { id: ChartPeriod; label: string }[] = $derived([
        { id: "day", label: tr("usage.period.byDay") },
        { id: "month", label: tr("usage.period.byMonth") },
        { id: "year", label: tr("usage.period.byYear") },
    ]);
</script>

{#snippet panelBody()}
    <header
        class="flex shrink-0 flex-col gap-3 border-b border-tr-border-strong px-5 py-4 sm:flex-row sm:items-center sm:justify-between"
    >
        <div class="flex min-w-0 items-center gap-2.5">
            <div
                class="flex shrink-0 items-center justify-center rounded-lg border border-tr-accent-soft-strong bg-tr-accent-soft p-1.5 text-tr-accent"
            >
                <ChartColumn size={16} strokeWidth={2} aria-hidden="true" />
            </div>
            <div class="min-w-0">
                <h2 class="text-base font-semibold text-white">
                    {tr("usage.title")}
                </h2>
            </div>
        </div>
        <div class="flex flex-wrap items-center gap-2 sm:justify-end">
            <div
                class="flex items-center gap-1.5 rounded-lg border border-tr-border-strong bg-tr-elevated px-2 py-1"
                role="group"
                aria-label={tr("usage.badge.dimensionLabel")}
            >
                <span class="text-[10px] text-tr-ink-3"
                    >{tr("usage.badge.label")}</span
                >
                <button
                    type="button"
                    class="rounded-md px-2 py-0.5 text-xs transition-colors {badgePeriod ===
                    'day'
                        ? 'bg-tr-accent text-white'
                        : 'text-tr-ink-2 hover:bg-tr-elevated'}"
                    aria-pressed={badgePeriod === "day"}
                    onclick={() => selectBadgePeriod("day")}
                >
                    {tr("usage.period.today")}
                </button>
                <button
                    type="button"
                    class="rounded-md px-2 py-0.5 text-xs transition-colors {badgePeriod ===
                    'month'
                        ? 'bg-tr-accent text-white'
                        : 'text-tr-ink-2 hover:bg-tr-elevated'}"
                    aria-pressed={badgePeriod === "month"}
                    onclick={() => selectBadgePeriod("month")}
                >
                    {tr("usage.period.month")}
                </button>
            </div>
            <button
                type="button"
                class="tr-press rounded-lg border border-tr-border-strong px-2.5 min-w-14 py-1.5 text-xs text-tr-ink-2 transition-colors hover:bg-tr-elevated disabled:opacity-40"
                disabled={loading}
                onclick={() => void refresh(true)}
            >
                {loading ? tr("common.refreshing") : tr("common.refresh")}
            </button>
            {#if !standalone}
                <CloseButton onclick={onclose} class="px-2.5 py-1.5 text-xs" />
            {/if}
        </div>
    </header>

    <div class="min-h-0 flex-1 overflow-y-auto px-5 py-4">
        {#if loading && !snapshot}
            <div class="flex items-center gap-2 py-8 text-sm text-tr-ink-3">
                <span
                    class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-tr-accent border-t-transparent"
                ></span>
                {tr("usage.loadingLogs")}
            </div>
        {:else if snapshot}
            {#if error}
                <div
                    class="mb-4 rounded-lg border border-tr-watch/30 bg-tr-watch-soft px-3 py-2 text-sm text-tr-watch"
                >
                    {error}
                </div>
            {/if}

            <div class="mb-5 grid gap-3 sm:grid-cols-3">
                {#each summaryCards as card}
                    {@const highlighted =
                        (badgePeriod === "day" &&
                            card.label === tr("usage.period.today")) ||
                        (badgePeriod === "month" &&
                            card.label === tr("usage.period.month"))}
                    <div
                        class="rounded-xl border p-3 {highlighted
                            ? 'border-tr-accent-soft-strong bg-tr-accent-soft'
                            : 'border-tr-border-strong bg-tr-elevated'}"
                    >
                        <p
                            class="text-[10px] uppercase tracking-wide text-tr-ink-3"
                        >
                            {card.label}
                            {#if highlighted}
                                <span
                                    class="ml-1 normal-case text-tr-accent"
                                    >{tr("usage.badge.marker")}</span
                                >
                            {/if}
                        </p>
                        <p class="mt-1 text-lg font-semibold text-white">
                            {formatCost(card.totals.total_cost_usd)}
                        </p>
                        <p class="mt-0.5 text-xs text-tr-ink-3">
                            {formatTokens(card.totals.total_tokens)}
                            {tr("usage.metric.tokens")}
                        </p>
                    </div>
                {/each}
            </div>

            <section class="mb-5">
                <h3
                    class="mb-2 text-xs font-medium uppercase tracking-wide text-tr-ink-3"
                >
                    {tr("usage.sources.title")}
                </h3>
                <ul class="space-y-2">
                    {#each snapshot.probe.sources as source}
                        <li
                            class="flex items-center justify-between gap-3 rounded-lg border border-tr-border-strong bg-tr-page px-3 py-2 text-xs"
                        >
                            <div class="flex min-w-0 items-center gap-2">
                                <span
                                    class="inline-block h-1.5 w-1.5 shrink-0 rounded-full {source.detected
                                        ? 'bg-tr-good'
                                        : 'bg-tr-raised'}"
                                ></span>
                                <span class="font-medium text-tr-ink-2"
                                    >{source.label}</span
                                >
                            </div>
                            <span
                                class="shrink-0 {source.detected
                                    ? 'text-tr-good'
                                    : 'text-tr-ink-3'}"
                            >
                                {source.detected
                                    ? tr("usage.sources.installed")
                                    : tr("usage.sources.notDetected")}
                            </span>
                        </li>
                    {/each}
                </ul>
                {#if snapshot.generated_at}
                    <p class="mt-2 text-[10px] text-tr-ink-3">
                        {#if snapshot.cached}{tr("usage.sources.cached")}{/if}{tr(
                            "usage.sources.updatedAt",
                            { time: formatTime(snapshot.generated_at) },
                        )}
                    </p>
                {/if}
            </section>

            <section class="mb-5">
                <div
                    class="mb-3 flex flex-wrap items-center justify-between gap-2"
                >
                    <div
                        class="flex gap-1 rounded-lg border border-tr-border-strong bg-tr-elevated p-0.5"
                    >
                        {#each chartPeriodOptions as option}
                            <button
                                type="button"
                                class="rounded-md px-3 py-1.5 text-xs transition-colors {chartPeriod ===
                                option.id
                                    ? 'bg-tr-accent text-white'
                                    : 'text-tr-ink-2 hover:bg-tr-elevated'}"
                                aria-pressed={chartPeriod === option.id}
                                onclick={() => (chartPeriod = option.id)}
                            >
                                {option.label}
                            </button>
                        {/each}
                    </div>
                    <div
                        class="flex gap-1 rounded-lg border border-tr-border-strong bg-tr-elevated p-0.5"
                    >
                        <button
                            type="button"
                            class="rounded-md px-2.5 py-1 text-xs transition-colors {chartMetric ===
                            'tokens'
                                ? 'bg-tr-elevated text-white'
                                : 'text-tr-ink-3 hover:bg-tr-elevated'}"
                            aria-pressed={chartMetric === "tokens"}
                            onclick={() => (chartMetric = "tokens")}
                        >
                            {tr("usage.metric.tokens")}
                        </button>
                        <button
                            type="button"
                            class="rounded-md px-2.5 py-1 text-xs transition-colors {chartMetric ===
                            'cost'
                                ? 'bg-tr-elevated text-white'
                                : 'text-tr-ink-3 hover:bg-tr-elevated'}"
                            aria-pressed={chartMetric === "cost"}
                            onclick={() => (chartMetric = "cost")}
                        >
                            {tr("usage.metric.cost")}
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
                loading={detailLoading}
                loadingLabel={tr("usage.detail.loadingSessions")}
                rowOpenPath={chartPeriod === "day" ? sessionOpenPath : undefined}
                onRowOpen={chartPeriod === "day" ? openSessionPath : undefined}
            />

            <p class="mt-4 text-[10px] leading-relaxed text-tr-ink-3">
                {tr("usage.footnote")}
            </p>
        {/if}
    </div>
{/snippet}

{#if standalone}
    <div
        class="flex h-screen flex-col bg-tr-page text-white"
        aria-label={tr("usage.title")}
    >
        {@render panelBody()}
    </div>
{:else}
    <SlideDrawer
        {open}
        {onclose}
        ariaLabel={tr("usage.title")}
        widthClass="w-[min(92vw,44rem)]"
    >
        {@render panelBody()}
    </SlideDrawer>
{/if}
