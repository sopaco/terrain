<script lang="ts">
    import type { FreshnessSummary } from "../types";
    import { tr } from "../i18n";
    import CloseButton from "./icons/CloseButton.svelte";
    import ModalShell from "./ModalShell.svelte";

    interface Props {
        open: boolean;
        freshness: FreshnessSummary | null;
        onclose: () => void;
        onQuickRefresh?: () => void;
        quickRefreshBusy?: boolean;
    }

    let {
        open,
        freshness,
        onclose,
        onQuickRefresh,
        quickRefreshBusy = false,
    }: Props = $props();

    const FRESH_THRESHOLD = 80;
    const VERIFY_THRESHOLD = 70;
    const MACRO_THRESHOLD = 50;

    const scoreBands = $derived([
        {
            min: FRESH_THRESHOLD,
            label: tr("freshness.bands.fresh"),
            tone: "text-tr-good",
            hint: tr("freshness.bands.freshHint"),
        },
        {
            min: VERIFY_THRESHOLD,
            label: tr("freshness.bands.verify"),
            tone: "text-tr-watch",
            hint: tr("freshness.bands.verifyHint"),
        },
        {
            min: MACRO_THRESHOLD,
            label: tr("freshness.bands.low"),
            tone: "text-tr-watch",
            hint: tr("freshness.bands.lowHint"),
        },
        {
            min: 0,
            label: tr("freshness.bands.stale"),
            tone: "text-tr-critical",
            hint: tr("freshness.bands.staleHint"),
        },
    ]);

    function severityStyle(severity: string): string {
        switch (severity) {
            case "high":
                return "border-tr-critical/30 bg-tr-critical-soft";
            case "medium":
                return "border-tr-watch/30 bg-tr-watch-soft";
            case "low":
                return "border-tr-border-strong bg-tr-elevated";
            default:
                return "border-tr-accent-soft-strong bg-tr-accent-soft";
        }
    }

    function severityLabel(severity: string): string {
        switch (severity) {
            case "high":
                return tr("freshness.severity.high");
            case "medium":
                return tr("freshness.severity.medium");
            case "low":
                return tr("freshness.severity.low");
            default:
                return tr("freshness.severity.info");
        }
    }

    function formatComputedAt(value?: string): string {
        if (!value) return "—";
        const date = new Date(value);
        if (Number.isNaN(date.getTime())) return value;
        return date.toLocaleString(undefined, {
            month: "short",
            day: "numeric",
            hour: "2-digit",
            minute: "2-digit",
        });
    }

    const negativeFactors = $derived(
        (freshness?.drift_factors ?? []).filter((f) => f.severity !== "info"),
    );
    const infoFactors = $derived(
        (freshness?.drift_factors ?? []).filter((f) => f.severity === "info"),
    );
</script>

<ModalShell
    {open}
    {onclose}
    ariaLabelledby="freshness-help-title"
    dialogClass="max-w-[min(92vw,560px)] max-h-[min(85vh,680px)]"
>
    <header
        class="flex shrink-0 items-start justify-between gap-3 border-b border-tr-border-strong px-5 py-4"
    >
        <div class="min-w-0">
            <h2
                id="freshness-help-title"
                class="text-base font-semibold text-tr-ink"
            >
                {tr("freshness.title")}
            </h2>
            <p class="mt-0.5 text-xs text-tr-ink-3">
                {#if freshness}
                    {tr("freshness.help.currentScorePrefix")}
                    <span class="font-medium text-tr-ink-2"
                        >{freshness.overall_score}/100</span
                    >
                    · {tr("freshness.help.updatedAt", {
                        time: formatComputedAt(freshness.last_computed_at),
                    })}
                {:else}
                    {tr("freshness.help.noData")}
                {/if}
            </p>
        </div>
        <CloseButton onclick={onclose} class="py-1 text-sm" />
    </header>

    <div class="flex-1 space-y-5 overflow-y-auto px-5 py-4">
        <section>
            <h3
                class="text-xs font-semibold uppercase tracking-wider text-tr-ink-3"
            >
                {tr("freshness.help.scoreMeaning")}
            </h3>
            <p class="mt-2 text-sm leading-relaxed text-tr-ink-2">
                {tr("freshness.help.scoreMeaningBody")}
            </p>
            <ul class="mt-3 space-y-2">
                {#each scoreBands as band}
                    <li class="flex items-start gap-2 text-xs">
                        <span
                            class={`mt-0.5 w-14 shrink-0 font-medium ${band.tone}`}
                            >≥{band.min}</span
                        >
                        <span class="text-tr-ink-3">{band.hint}</span>
                    </li>
                {/each}
            </ul>
        </section>

        <section
            class="rounded-xl border border-tr-border bg-tr-elevated px-4 py-3"
        >
            <h3
                class="text-xs font-semibold uppercase tracking-wider text-tr-ink-3"
            >
                {tr("freshness.help.howCalculated")}
            </h3>
            <p class="mt-2 text-xs leading-relaxed text-tr-ink-2">
                {tr("freshness.help.calcBody1")}<strong
                    class="font-medium text-tr-ink-2"
                    >{tr("freshness.help.calcBodyStrong")}</strong
                >{tr("freshness.help.calcBody2")}
            </p>
            <ul
                class="mt-3 list-inside list-disc space-y-1.5 text-xs text-tr-ink-3"
            >
                <li>
                    <span class="text-tr-ink-2"
                        >{tr("freshness.help.sourceIndex")}</span
                    >{tr("freshness.help.sourceIndexDesc")}
                </li>
                <li>
                    <span class="text-tr-ink-2">{tr("terms.agentKnowledge")}</span
                    >{tr("freshness.help.agentKnowledgeDesc")}
                </li>
                <li>
                    <span class="text-tr-ink-2">{tr("terms.humanKnowledge")}</span>
                    {tr("freshness.help.humanKnowledgeDesc")}
                </li>
            </ul>
            <div
                class="mt-3 grid gap-2 text-[11px] text-tr-ink-3 sm:grid-cols-2"
            >
                <div
                    class="rounded-lg border border-tr-border bg-tr-page px-3 py-2"
                >
                    {tr("freshness.help.penaltyCommit")}
                </div>
                <div
                    class="rounded-lg border border-tr-border bg-tr-page px-3 py-2"
                >
                    {tr("freshness.help.penaltyFiles")}
                </div>
                <div
                    class="rounded-lg border border-tr-border bg-tr-page px-3 py-2"
                >
                    {tr("freshness.help.penaltyDays")}
                </div>
                <div
                    class="rounded-lg border border-tr-border bg-tr-page px-3 py-2"
                >
                    {tr("freshness.help.penaltyDirty")}
                </div>
            </div>
        </section>

        {#if freshness}
            <section>
                <h3
                    class="text-xs font-semibold uppercase tracking-wider text-tr-ink-3"
                >
                    {tr("freshness.help.layerScores")}
                </h3>
                <div class="mt-2 grid gap-2 sm:grid-cols-3">
                    <div
                        class="rounded-xl border border-tr-border-strong bg-tr-page px-3 py-2.5"
                    >
                        <p class="text-[10px] text-tr-ink-3">
                            {tr("freshness.help.sourceIndex")}
                        </p>
                        <p class="text-lg font-semibold text-tr-ink">
                            {freshness.agent_pack_score}
                        </p>
                        {#if freshness.pack_baseline_short}
                            <p class="text-[10px] text-tr-ink-3">
                                baseline {freshness.pack_baseline_short}
                            </p>
                        {/if}
                    </div>
                    <div
                        class="rounded-xl border border-tr-border-strong bg-tr-page px-3 py-2.5"
                    >
                        <p class="text-[10px] text-tr-ink-3">
                            {tr("freshness.help.agentContextLayer")}
                        </p>
                        <p class="text-lg font-semibold text-tr-ink">
                            {freshness.agent_context_score}
                        </p>
                        {#if freshness.context_baseline_short}
                            <p class="text-[10px] text-tr-ink-3">
                                baseline {freshness.context_baseline_short}
                            </p>
                        {/if}
                    </div>
                    <div
                        class="rounded-xl border border-tr-border-strong bg-tr-page px-3 py-2.5"
                    >
                        <p class="text-[10px] text-tr-ink-3">
                            {tr("freshness.help.humanDocsLayer")}
                        </p>
                        <p class="text-lg font-semibold text-tr-ink">
                            {freshness.human_docs_score}
                        </p>
                    </div>
                </div>
            </section>

            <section>
                <h3
                    class="text-xs font-semibold uppercase tracking-wider text-tr-ink-3"
                >
                    {#if negativeFactors.length > 0}
                        {tr("freshness.help.driftReasons")}
                    {:else}
                        {tr("freshness.help.currentStatus")}
                    {/if}
                </h3>
                {#if negativeFactors.length === 0 && infoFactors.length === 0}
                    <p class="mt-2 text-sm text-tr-ink-3">
                        {tr("freshness.help.noAnalysis")}
                    </p>
                {:else}
                    <ul class="mt-2 space-y-2">
                        {#each negativeFactors as factor}
                            <li
                                class={`rounded-xl border px-3.5 py-3 ${severityStyle(factor.severity)}`}
                            >
                                <div class="flex flex-wrap items-center gap-2">
                                    <span
                                        class="text-sm font-medium text-tr-ink"
                                        >{factor.title}</span
                                    >
                                    <span
                                        class="rounded-full bg-tr-page px-1.5 py-0.5 text-[10px] text-tr-ink-3"
                                    >
                                        {severityLabel(factor.severity)}
                                    </span>
                                    {#if factor.points_lost != null && factor.points_lost > 0}
                                        <span
                                            class="text-[10px] text-tr-critical"
                                            >{tr("freshness.help.pointsLost", {
                                                points: factor.points_lost,
                                            })}</span
                                        >
                                    {/if}
                                </div>
                                <p
                                    class="mt-1.5 text-xs leading-relaxed text-tr-ink-2"
                                >
                                    {factor.detail}
                                </p>
                            </li>
                        {/each}
                    </ul>
                {/if}

                {#if freshness.sample_changed_files?.length}
                    <div
                        class="mt-3 rounded-xl border border-tr-border bg-tr-page px-3 py-2.5"
                    >
                        <p
                            class="text-[10px] font-semibold uppercase tracking-wider text-tr-ink-3"
                        >
                            {tr("freshness.help.changedFilesList", {
                                count: freshness.changed_files_count,
                            })}
                        </p>
                        <ul
                            class="mt-2 max-h-28 space-y-0.5 overflow-y-auto font-mono text-[11px] text-tr-ink-3"
                        >
                            {#each freshness.sample_changed_files as path}
                                <li class="truncate" title={path}>{path}</li>
                            {/each}
                        </ul>
                    </div>
                {/if}

                {#if infoFactors.length > 0}
                    <ul class="mt-3 space-y-2">
                        {#each infoFactors as factor}
                            <li
                                class={`rounded-xl border px-3.5 py-2.5 ${severityStyle(factor.severity)}`}
                            >
                                <p class="text-xs font-medium text-tr-accent">
                                    {factor.title}
                                </p>
                                <p
                                    class="mt-1 text-xs leading-relaxed text-tr-ink-3"
                                >
                                    {factor.detail}
                                </p>
                            </li>
                        {/each}
                    </ul>
                {/if}
            </section>

            <section
                class="rounded-xl border border-tr-accent-soft-strong bg-tr-accent-soft px-4 py-3"
            >
                <h3 class="text-xs font-semibold text-tr-accent">
                    {tr("freshness.help.whatToDo")}
                </h3>
                <ul
                    class="mt-2 list-inside list-disc space-y-1 text-xs leading-relaxed text-tr-ink-2"
                >
                    {#if freshness.working_tree_dirty}
                        <li>
                            {tr("freshness.help.tipCommitBefore")}<strong
                                >{tr("freshness.help.tipCommitStrong")}</strong
                            >{tr("freshness.help.tipCommitAfter")}`.terrain/`
                            {tr("freshness.help.tipCommitSuffix")}
                        </li>
                    {/if}
                    {#if freshness.commits_since_baseline > 0 || freshness.changed_files_count > 0}
                        <li>
                            {tr("freshness.help.tipQuickRefresh")}
                        </li>
                    {/if}
                    {#if freshness.overall_stale}
                        <li>
                            {tr("freshness.help.tipLowScore")}
                        </li>
                    {:else}
                        <li>{tr("freshness.help.tipKeepUp")}</li>
                    {/if}
                </ul>
                {#if onQuickRefresh}
                    <button
                        type="button"
                        class="tr-press mt-3 rounded-lg bg-tr-accent px-3 py-1.5 text-xs font-medium transition-colors hover:bg-tr-accent-hover disabled:opacity-50"
                        disabled={quickRefreshBusy}
                        onclick={() => {
                            onQuickRefresh();
                            onclose();
                        }}
                    >
                        {quickRefreshBusy
                            ? tr("freshness.refreshing")
                            : tr("freshness.quickRefreshNow")}
                    </button>
                {/if}
            </section>
        {/if}
    </div>
</ModalShell>
