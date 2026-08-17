<script lang="ts">
    import { Check, CircleCheck, Copy, FolderOpen } from "@lucide/svelte";
    import type { ProjectOverview, ProjectRegistryEntry } from "../types";
    import { copyTextToClipboard } from "../clipboard";
    import {
        registryDisplayName,
        registryRepairDetail,
    } from "../projectRegistry";
    import { tr } from "../i18n";
    import FreshnessHelpPanel from "./FreshnessHelpPanel.svelte";
    import OverviewActionBanner, {
        type OverviewActionItem,
    } from "./OverviewActionBanner.svelte";
    import OverviewKnowledgeCard from "./OverviewKnowledgeCard.svelte";
    import ReadinessHelpPanel from "./ReadinessHelpPanel.svelte";
    import HelpButton from "./icons/HelpButton.svelte";

    interface Props {
        overview: ProjectOverview | null;
        loading: boolean;
        acpOk: boolean;
        llmReady: boolean;
        hybridNativeLlm?: boolean;
        agentContextBusy?: boolean;
        lithoBusy?: boolean;
        repackBusy?: boolean;
        initBusy?: boolean;
        initProgress?: string | null;
        registryProjects?: ProjectRegistryEntry[];
        selectedRegistry?: ProjectRegistryEntry | null;
        onOpenKnowledge: () => void;
        onOpenEnv?: () => void;
        onOpenAsk: () => void;
        onOpenSettings?: () => void;
        onGenerateHuman?: () => void;
        onGenerateAgentContext?: () => void;
        onRepack?: () => void;
        onInitializeProject?: (repoPath: string, slug?: string) => void;
        onOpenPath?: (path: string) => void;
        onOpenArchitectureDoc?: () => void;
        onOpenHumanOverview?: () => void;
        onOpenStructured?: () => void;
        quickRefreshBusy?: boolean;
        freshnessLoading?: boolean;
        onQuickRefresh?: () => void;
        onRequestFreshnessCompute?: () => void;
        onSaveProjectRemark?: (remark: string) => Promise<void>;
    }

    let {
        overview,
        loading,
        acpOk,
        llmReady,
        hybridNativeLlm = false,
        agentContextBusy = false,
        lithoBusy = false,
        repackBusy = false,
        initBusy = false,
        initProgress = null,
        registryProjects = [],
        selectedRegistry = null,
        onOpenKnowledge,
        onOpenEnv,
        onOpenAsk,
        onOpenSettings,
        onGenerateHuman,
        onGenerateAgentContext,
        onRepack,
        onInitializeProject,
        onOpenPath,
        onOpenArchitectureDoc,
        onOpenHumanOverview,
        onOpenStructured,
        quickRefreshBusy = false,
        freshnessLoading = false,
        onQuickRefresh,
        onRequestFreshnessCompute,
        onSaveProjectRemark,
    }: Props = $props();

    let freshnessHelpOpen = $state(false);
    let readinessHelpOpen = $state(false);
    let remarkEditing = $state(false);
    let remarkDraft = $state("");
    let remarkSaving = $state(false);
    let copiedPath = $state<string | null>(null);

    $effect(() => {
        if (freshnessHelpOpen) onRequestFreshnessCompute?.();
    });

    const readyCount = $derived(
        overview?.asset_health.filter((a) => a.ready).length ?? 0,
    );

    const assetTotal = $derived(overview?.asset_health.length ?? 0);

    const readinessPercent = $derived(
        assetTotal > 0 ? Math.round((readyCount / assetTotal) * 100) : 0,
    );

    const assetChecklist = $derived(
        overview?.asset_health.map((a) => ({
            label: a.label,
            ready: a.ready,
        })) ?? [],
    );

    const structuredAsset = $derived(
        overview?.asset_health.find((a) => a.track === "structured") ?? null,
    );

    const needsAssetInit = $derived(
        overview != null && readyCount < assetTotal,
    );

    const needsEnvSetup = $derived(
        overview != null && !overview.agent_env.ready,
    );

    const initHint = $derived.by(() => {
        const needsLlm = hybridNativeLlm && !llmReady;
        const needsAcp = !acpOk;
        if (!needsLlm && !needsAcp) return null;
        const parts: string[] = [];
        if (needsLlm)
            parts.push(
                tr("overview.initHint.needsLlm", {
                    term: tr("terms.agentKnowledge"),
                }),
            );
        if (needsAcp)
            parts.push(
                tr("overview.initHint.needsAcp", {
                    term: tr("terms.humanKnowledge"),
                }),
            );
        return tr("overview.initHint.configRequired", {
            parts: parts.join("、"),
        });
    });

    const knowledgePath = $derived(
        overview?.repo_path ? `${overview.repo_path}/.terrain` : null,
    );

    const freshness = $derived(overview?.freshness ?? null);

    const freshnessScore = $derived(freshness?.overall_score ?? null);

    function freshnessTone(score: number): "good" | "watch" | "critical" {
        if (score >= 80) return "good";
        if (score >= 50) return "watch";
        return "critical";
    }

    const needsGenerationSetup = $derived(
        (hybridNativeLlm && !llmReady) || !acpOk,
    );

    const actionItems = $derived.by((): OverviewActionItem[] => {
        if (!overview) return [];

        const items: OverviewActionItem[] = [];

        if (freshness?.overall_stale && overview.repo_path && onQuickRefresh) {
            const driftParts: string[] = [
                tr("freshness.score", { score: freshness.overall_score }),
            ];
            if (freshness.commits_since_baseline > 0) {
                driftParts.push(
                    tr("freshness.behind", {
                        count: freshness.commits_since_baseline,
                    }),
                );
            }
            if (freshness.changed_files_count > 0) {
                driftParts.push(
                    tr("freshness.changedFiles", {
                        count: freshness.changed_files_count,
                    }),
                );
            }
            if (freshness.working_tree_dirty) {
                driftParts.push(tr("freshness.dirtyTree"));
            }
            items.push({
                id: "stale",
                priority: 1,
                accent: "rose",
                title: tr("overview.actions.staleTitle"),
                detail: tr("overview.actions.staleDetail", {
                    drift: driftParts.join(" · "),
                }),
                hint: tr("overview.actions.staleHint"),
                actionLabel: tr("overview.actions.quickRefresh"),
                busyLabel: tr("overview.actions.quickRefreshing"),
                onAction: onQuickRefresh,
                disabled: initBusy,
                busy: quickRefreshBusy,
            });
        }

        if (needsAssetInit && overview.repo_path && onInitializeProject) {
            items.push({
                id: "init",
                priority: 2,
                accent: "amber",
                title: tr("overview.actions.initTitle"),
                detail: tr("overview.actions.initDetail", {
                    ready: readyCount,
                    total: assetTotal,
                    agentKnowledge: tr("terms.agentKnowledge"),
                    humanKnowledge: tr("terms.humanKnowledge"),
                }),
                hint: initHint ?? undefined,
                actionLabel: tr("overview.actions.init"),
                busyLabel: tr("overview.actions.initializing"),
                onAction: () =>
                    onInitializeProject(overview.repo_path, overview.slug),
                disabled: initBusy,
                busy: initBusy,
            });
        }

        if (needsEnvSetup && onOpenEnv) {
            items.push({
                id: "env",
                priority: 3,
                accent: "violet",
                title: tr("overview.actions.envTitle", {
                    term: tr("terms.agentEnv"),
                }),
                detail: tr("overview.actions.envDetail", {
                    summary: overview.agent_env.summary,
                }),
                actionLabel: tr("overview.actions.envConfigure"),
                onAction: onOpenEnv,
            });
        }

        return items;
    });

    const staleRegistryProjects = $derived(
        registryProjects.filter((p) => p.status === "stale"),
    );

    const staleActionItems = $derived.by((): OverviewActionItem[] =>
        staleRegistryProjects.map((entry) => ({
            id: `stale-${entry.slug}`,
            priority: 2,
            accent: "amber" as const,
            title: registryDisplayName(entry),
            detail: registryRepairDetail(entry),
            hint: initHint ?? undefined,
            actionLabel: tr("overview.actions.reinit"),
            busyLabel: tr("overview.actions.reinitializing"),
            onAction: onInitializeProject
                ? () => onInitializeProject(entry.repo_path, entry.slug)
                : undefined,
            disabled: initBusy || !onInitializeProject,
            busy: initBusy,
        })),
    );

    const selectedStaleActionItems = $derived.by((): OverviewActionItem[] => {
        if (!selectedRegistry || selectedRegistry.status !== "stale") return [];
        return [
            {
                id: `stale-selected-${selectedRegistry.slug}`,
                priority: 1,
                accent: "amber" as const,
                title: tr("overview.actions.dataLostTitle"),
                detail: registryRepairDetail(selectedRegistry),
                hint: initHint ?? undefined,
                actionLabel: tr("overview.actions.reinit"),
                busyLabel: tr("overview.actions.reinitializing"),
                onAction: onInitializeProject
                    ? () =>
                          onInitializeProject(
                              selectedRegistry.repo_path,
                              selectedRegistry.slug,
                          )
                    : undefined,
                disabled: initBusy || !onInitializeProject,
                busy: initBusy,
            },
        ];
    });

    function freshnessBadgeClass(
        score: number,
        stale?: boolean | null,
    ): string {
        if (!stale && score >= 80) return "bg-tr-good-soft text-tr-good";
        if (score >= 50) return "bg-tr-watch-soft text-tr-watch";
        return "bg-tr-critical-soft text-tr-critical";
    }

    function formatSyncedAt(value?: string | null): string {
        if (!value) return "—";
        const date = new Date(value);
        if (Number.isNaN(date.getTime())) return value;
        return date.toLocaleString(undefined, {
            year: "numeric",
            month: "2-digit",
            day: "2-digit",
            hour: "2-digit",
            minute: "2-digit",
        });
    }

    function humanKnowledgeMeta(o: ProjectOverview): string {
        if (o.litho.human_docs_complete) {
            return tr("overview.meta.docCount", {
                count: o.litho.human_doc_count,
            });
        }
        if (o.litho.has_human_docs) {
            return tr("overview.meta.docCountIncomplete", {
                count: o.litho.human_doc_count,
            });
        }
        if (o.litho.has_research_artifacts) {
            return tr("overview.meta.researchReady");
        }
        return tr("overview.meta.notGenerated");
    }

    function assetPrimaryAction(
        asset: ProjectOverview["asset_health"][number],
    ): {
        label: string;
        onClick?: () => void;
        disabled?: boolean;
    } | null {
        if (asset.track === "human") {
            if (asset.ready && onOpenHumanOverview) {
                return {
                    label: tr("overview.actions.browse"),
                    onClick: onOpenHumanOverview,
                };
            }
            if (onGenerateHuman) {
                return {
                    label: lithoBusy
                        ? tr("terms.generating")
                        : tr("terms.generate", {
                              term: tr("terms.humanKnowledge"),
                          }),
                    onClick: onGenerateHuman,
                    disabled: lithoBusy || !acpOk,
                };
            }
        } else if (asset.track === "agent_context") {
            if (asset.ready && onOpenArchitectureDoc) {
                return {
                    label: tr("overview.actions.browse"),
                    onClick: onOpenArchitectureDoc,
                };
            }
            if (onGenerateAgentContext) {
                return {
                    label: asset.ready
                        ? agentContextBusy
                            ? tr("common.generating")
                            : tr("common.regenerate")
                        : tr("terms.generate", {
                              term: tr("terms.agentKnowledge"),
                          }),
                    onClick: onGenerateAgentContext,
                    disabled: agentContextBusy || !llmReady,
                };
            }
        } else if (asset.track === "agent_pack" && onRepack) {
            return {
                label: repackBusy
                    ? tr("terms.msg.repacking")
                    : tr("terms.msg.repack"),
                onClick: onRepack,
                disabled: repackBusy,
            };
        } else if (asset.track === "structured") {
            if (asset.ready && onOpenStructured) {
                return {
                    label: tr("overview.actions.browse"),
                    onClick: onOpenStructured,
                };
            }
            return {
                label: tr("overview.actions.scanProject"),
                onClick: onOpenKnowledge,
            };
        }
        return null;
    }

    const assetRows = $derived(
        overview?.asset_health.map((asset) => ({
            asset,
            action: assetPrimaryAction(asset),
        })) ?? [],
    );

    function startRemarkEdit() {
        remarkDraft = overview?.project_remark ?? "";
        remarkEditing = true;
    }

    function cancelRemarkEdit() {
        remarkEditing = false;
        remarkDraft = overview?.project_remark ?? "";
    }

    async function saveRemark() {
        if (!onSaveProjectRemark) return;
        remarkSaving = true;
        try {
            await onSaveProjectRemark(remarkDraft);
            remarkEditing = false;
        } finally {
            remarkSaving = false;
        }
    }

    async function copyPath(path: string) {
        try {
            await copyTextToClipboard(path);
            copiedPath = path;
            window.setTimeout(() => {
                if (copiedPath === path) copiedPath = null;
            }, 1500);
        } catch {
            /* clipboard unavailable — silently ignore */
        }
    }
</script>

{#snippet metaRow(label: string, path: string)}
    <div class="flex items-center gap-2 text-[11.5px]">
        <span class="w-16 shrink-0 text-tr-ink-3">{label}</span>
        <span
            class="min-w-0 flex-1 truncate font-mono text-tr-ink-2"
            title={path}>{path}</span
        >
        <button
            type="button"
            class="tr-press shrink-0 rounded-md p-1 text-tr-ink-3 transition-colors hover:bg-tr-elevated hover:text-tr-ink"
            title={tr("common.copy")}
            aria-label={tr("overview.copyLabel", { label })}
            onclick={() => copyPath(path)}
        >
            {#if copiedPath === path}
                <Check
                    size={12}
                    strokeWidth={2.5}
                    class="text-tr-good"
                    aria-hidden="true"
                />
            {:else}
                <Copy size={12} strokeWidth={2} aria-hidden="true" />
            {/if}
        </button>
        {#if onOpenPath}
            <button
                type="button"
                class="tr-press shrink-0 rounded-md p-1 text-tr-ink-3 transition-colors hover:bg-tr-elevated hover:text-tr-ink"
                title={tr("overview.openInFinder")}
                aria-label={tr("overview.openLabel", { label })}
                onclick={() => onOpenPath(path)}
            >
                <FolderOpen size={12} strokeWidth={2} aria-hidden="true" />
            </button>
        {/if}
    </div>
{/snippet}

<div class="flex h-full flex-col overflow-y-auto bg-tr-page">
    {#if loading}
        <div
            class="flex flex-1 flex-col items-center justify-center gap-3 text-sm text-tr-ink-3"
        >
            <span
                class="inline-block h-8 w-8 animate-spin rounded-full border-2 border-tr-accent border-t-transparent"
            ></span>
            <span>{tr("overview.loading")}</span>
        </div>
    {:else if selectedRegistry?.status === "stale"}
        <div
            class="mx-auto flex w-full max-w-3xl flex-1 flex-col gap-6 px-6 py-10"
        >
            <header class="space-y-3">
                <div class="flex flex-wrap items-center gap-2">
                    <h2 class="text-xl font-semibold text-tr-ink">
                        {registryDisplayName(selectedRegistry)}
                    </h2>
                    <span
                        class="rounded-full bg-tr-watch-soft px-2 py-0.5 text-[11px] font-medium text-tr-watch"
                        >{tr("overview.needsRepair")}</span
                    >
                </div>
                <p class="font-mono text-xs text-tr-ink-3">
                    {selectedRegistry.repo_path}
                </p>
                <p class="text-sm leading-relaxed text-tr-ink-2">
                    {tr("overview.staleIndex")}
                    <code class="text-tr-ink">index.md</code>
                    {tr("overview.staleMissingOr")}
                    <code class="text-tr-ink">.terrain/</code>
                    {tr("overview.staleCorrupted")}
                </p>
            </header>

            <OverviewActionBanner
                items={selectedStaleActionItems}
                progressNote={initBusy ? initProgress : null}
            />

            {#if selectedRegistry.repo_path && onOpenPath}
                <div class="flex items-center gap-3 pt-1">
                    <button
                        type="button"
                        class="tr-press inline-flex items-center gap-2 rounded-xl border border-tr-border-strong px-4 py-2 text-sm text-tr-ink-2 transition-colors hover:bg-tr-elevated"
                        onclick={() => onOpenPath(selectedRegistry.repo_path)}
                    >
                        <FolderOpen size={15} strokeWidth={2} />
                        {tr("overview.openRepo")}
                    </button>
                </div>
            {/if}
        </div>
    {:else if !overview}
        <div
            class="mx-auto flex w-full max-w-3xl flex-1 flex-col justify-center gap-6 px-6 py-10"
        >
            <section
                class="rounded-2xl border border-tr-border bg-tr-surface px-6 py-8"
            >
                <h2 class="text-xl font-semibold text-tr-ink">
                    {tr("overview.welcomeTitle")}
                </h2>
                <p class="mt-2 text-sm leading-relaxed text-tr-ink-2">
                    {tr("overview.welcomeBody")}
                </p>
                {#if staleRegistryProjects.length === 0}
                    <p class="mt-4 text-xs text-tr-ink-3">
                        {tr("overview.welcomeHint")}
                    </p>
                {/if}
            </section>

            {#if staleRegistryProjects.length > 0}
                <section class="space-y-3">
                    <h3 class="text-sm font-medium text-tr-ink-2">
                        {tr("overview.staleDetected")}
                    </h3>
                    <OverviewActionBanner
                        items={staleActionItems}
                        progressNote={initBusy ? initProgress : null}
                        collapseSecondary={false}
                    />
                </section>
            {/if}
        </div>
    {:else}
        <div class="mx-auto w-full max-w-6xl space-y-7 px-6 py-8">
            <!-- Project header -->
            <header class="flex flex-wrap items-start justify-between gap-4">
                <div class="min-w-0 flex-1">
                    <h2
                        class="break-words text-xl font-semibold tracking-tight text-tr-ink"
                    >
                        {overview.name}
                    </h2>

                    {#if remarkEditing}
                        <div class="mt-2 space-y-2">
                            <textarea
                                class="w-full resize-y rounded-xl border border-tr-border-strong bg-tr-surface px-3 py-2 text-sm text-tr-ink placeholder:text-tr-ink-3 focus:border-tr-accent focus:outline-none"
                                rows="2"
                                placeholder={tr("overview.remarkPlaceholder")}
                                bind:value={remarkDraft}
                                disabled={remarkSaving}></textarea>
                            <div class="flex flex-wrap gap-2">
                                <button
                                    type="button"
                                    class="tr-press rounded-lg bg-tr-accent px-3 py-1.5 text-xs font-medium text-tr-on-accent transition-colors hover:bg-tr-accent-hover disabled:opacity-50"
                                    disabled={remarkSaving ||
                                        !onSaveProjectRemark}
                                    onclick={saveRemark}
                                >
                                    {remarkSaving
                                        ? tr("common.saving")
                                        : tr("overview.saveRemark")}
                                </button>
                                <button
                                    type="button"
                                    class="tr-press rounded-lg border border-tr-border-strong px-3 py-1.5 text-xs text-tr-ink-2 transition-colors hover:bg-tr-elevated"
                                    disabled={remarkSaving}
                                    onclick={cancelRemarkEdit}
                                >
                                    {tr("common.cancel")}
                                </button>
                            </div>
                        </div>
                    {:else}
                        <div class="mt-2 flex items-center gap-2">
                            {#if overview.project_remark}
                                <span
                                    class="inline-flex max-w-full items-center gap-1.5 truncate rounded-full border border-tr-border bg-tr-elevated px-2.5 py-1 text-xs text-tr-ink-2"
                                >
                                    {overview.project_remark}
                                </span>
                            {:else}
                                <p class="text-sm text-tr-ink-3">
                                    {tr("overview.remarkEmpty")}
                                </p>
                            {/if}
                            {#if onSaveProjectRemark}
                                <button
                                    type="button"
                                    class="shrink-0 text-[11px] text-tr-ink-3 transition-colors hover:text-tr-accent"
                                    onclick={startRemarkEdit}
                                >
                                    {tr("common.edit")}
                                </button>
                            {/if}
                        </div>
                    {/if}

                    <p
                        class="mt-2 flex flex-wrap items-center gap-1.5 text-xs text-tr-ink-3 [font-variant-numeric:tabular-nums]"
                    >
                        <span
                            >{tr("overview.lastSynced", {
                                time: formatSyncedAt(overview.synced_at),
                            })}</span
                        >
                        {#if overview.collectors.length}
                            <span class="text-tr-ink-4">·</span>
                            <span>{overview.collectors.join(" · ")}</span>
                        {/if}
                        <span class="text-tr-ink-4">·</span>
                        <span class="font-mono text-tr-ink-3"
                            >{overview.slug}</span
                        >
                        {#if freshness?.current_git_head}
                            <span class="text-tr-ink-4">·</span>
                            <code
                                class="rounded bg-tr-elevated px-1.5 py-0.5 font-mono text-[11px] text-tr-ink-2"
                                >{freshness.current_git_head}</code
                            >
                        {/if}
                        {#if freshness?.working_tree_dirty}
                            <span class="text-tr-ink-4">·</span>
                            <span class="text-tr-watch"
                                >{tr("freshness.dirtyTree")}</span
                            >
                        {/if}
                    </p>
                </div>
                <div class="flex flex-wrap gap-2">
                    {#if overview.repo_path && onOpenPath}
                        <button
                            type="button"
                            class="tr-press rounded-xl border border-tr-border-strong px-4 py-2 text-sm text-tr-ink-2 transition-colors hover:bg-tr-elevated"
                            onclick={() => onOpenPath(overview.repo_path)}
                        >
                            {tr("overview.openRepo")}
                        </button>
                    {/if}
                    <button
                        type="button"
                        class="tr-press rounded-xl bg-tr-accent px-4 py-2 text-sm font-medium text-tr-on-accent transition-colors hover:bg-tr-accent-hover"
                        onclick={onOpenAsk}
                    >
                        {tr("overview.ask")}
                    </button>
                </div>
            </header>

            {#if actionItems.length > 0}
                <OverviewActionBanner
                    items={actionItems}
                    progressNote={initBusy &&
                    !actionItems.some((i) => i.id === "init")
                        ? initProgress
                        : null}
                />
            {/if}

            <!-- 状态一览：就绪度 + 新鲜度 -->
            <div class="grid gap-3 sm:grid-cols-2">
                <div
                    class="relative overflow-hidden rounded-xl border border-tr-border bg-tr-surface py-4 pl-5 pr-4"
                >
                    <span
                        class={`absolute inset-y-0 left-0 w-[3px] ${readyCount >= assetTotal ? "bg-tr-good" : "bg-tr-watch"}`}
                        aria-hidden="true"
                    ></span>
                    <div class="flex items-start justify-between gap-2">
                        <div class="flex items-center gap-1">
                            <span class="text-xs text-tr-ink-2"
                                >{tr("overview.readiness")}</span
                            >
                            <HelpButton
                                onclick={() => (readinessHelpOpen = true)}
                                title={tr("overview.readinessHelpTitle")}
                                ariaLabel={tr("overview.readinessAria")}
                                size={14}
                            />
                        </div>
                        <span
                            class="text-2xl font-semibold [font-variant-numeric:tabular-nums] text-tr-ink"
                        >
                            {readyCount}<span
                                class="text-sm font-normal text-tr-ink-3"
                                >/{assetTotal}</span
                            >
                        </span>
                    </div>
                    <div class="mt-3 flex flex-wrap gap-1.5">
                        {#each assetChecklist as item}
                            <span
                                class={`inline-flex items-center gap-1 rounded-md px-2 py-1 text-[11px] ${
                                    item.ready
                                        ? "bg-tr-good-soft text-tr-good"
                                        : "bg-tr-elevated text-tr-ink-3"
                                }`}
                            >
                                {#if item.ready}
                                    <CircleCheck
                                        size={11}
                                        strokeWidth={2}
                                        aria-hidden="true"
                                    />
                                {/if}
                                {item.label}
                            </span>
                        {/each}
                    </div>
                </div>

                <div
                    class="relative overflow-hidden rounded-xl border border-tr-border bg-tr-surface py-4 pl-5 pr-4"
                >
                    <span
                        class={`absolute inset-y-0 left-0 w-[3px] ${
                            freshnessScore == null
                                ? "bg-tr-border-strong"
                                : freshnessTone(freshnessScore) === "good"
                                  ? "bg-tr-good"
                                  : freshnessTone(freshnessScore) === "watch"
                                    ? "bg-tr-watch"
                                    : "bg-tr-critical"
                        }`}
                        aria-hidden="true"
                    ></span>
                    <div class="flex items-start justify-between gap-2">
                        <div class="flex items-center gap-1">
                            <span class="text-xs text-tr-ink-2"
                                >{tr("freshness.name")}</span
                            >
                            <HelpButton
                                onclick={() => (freshnessHelpOpen = true)}
                                title={tr("freshness.helpButtonTitle")}
                                ariaLabel={tr("freshness.title")}
                                size={14}
                            />
                        </div>
                        {#if freshnessLoading && !freshness}
                            <span class="text-xs text-tr-ink-3"
                                >{tr("freshness.computing")}</span
                            >
                        {:else if freshnessScore != null}
                            <span
                                class={`text-2xl font-semibold [font-variant-numeric:tabular-nums] ${
                                    freshnessTone(freshnessScore) === "good"
                                        ? "text-tr-good"
                                        : freshnessTone(freshnessScore) ===
                                            "watch"
                                          ? "text-tr-watch"
                                          : "text-tr-critical"
                                }`}
                            >
                                {freshnessScore}
                                <span class="text-sm font-normal text-tr-ink-3"
                                    >/100</span
                                >
                            </span>
                        {:else}
                            <span class="text-sm text-tr-ink-3">—</span>
                        {/if}
                    </div>

                    {#if freshnessLoading && !freshness}
                        <div
                            class="mt-3 h-2.5 animate-pulse rounded-full bg-tr-elevated"
                            role="status"
                            aria-live="polite"
                        ></div>
                    {:else if freshnessScore != null}
                        <div
                            class="mt-3 h-2.5 overflow-hidden rounded-full bg-tr-elevated"
                        >
                            <div
                                class={`h-full w-full origin-left rounded-full transition-transform duration-300 ease-out ${
                                    freshnessTone(freshnessScore) === "good"
                                        ? "bg-tr-good"
                                        : freshnessTone(freshnessScore) ===
                                            "watch"
                                          ? "bg-tr-watch"
                                          : "bg-tr-critical"
                                }`}
                                style={`transform: scaleX(${freshnessScore / 100})`}
                            ></div>
                        </div>
                        <p class="mt-2 text-[11px] text-tr-ink-3">
                            {#if freshnessLoading}
                                {tr("freshness.updating")}
                            {:else if freshness?.is_git_repo && freshness.current_git_head}
                                {tr("freshness.gitBased", {
                                    head: freshness.current_git_head,
                                })}
                                {#if freshness.working_tree_dirty}
                                    · {tr("freshness.dirtyTree")}
                                {/if}
                            {:else if freshness && !freshness.is_git_repo}
                                {tr("freshness.noGit")}
                            {/if}
                        </p>
                    {/if}
                </div>
            </div>

            <!-- 知识资产域 -->
            <section class="space-y-3">
                <div>
                    <h3 class="text-sm font-medium text-tr-ink-2">
                        {tr("terms.knowledgeTab")}
                    </h3>
                    <p class="mt-0.5 text-xs text-tr-ink-3">
                        {tr("overview.assetsDesc1")}
                        <code class="text-tr-ink-2">.terrain/</code>
                        {tr("overview.assetsDesc2")}
                    </p>
                </div>

                <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                    <OverviewKnowledgeCard
                        title={tr("terms.short.humanKnowledge")}
                        subtitle={tr("overview.card.humanSubtitle")}
                        meta={humanKnowledgeMeta(overview)}
                        ready={overview.litho.human_docs_complete}
                        icon="book"
                        featured={true}
                        primaryLabel={overview.litho.human_docs_complete
                            ? tr("common.open")
                            : lithoBusy
                              ? tr("terms.generating")
                              : tr("terms.generate", {
                                    term: tr("terms.humanKnowledge"),
                                })}
                        onPrimary={overview.litho.human_docs_complete
                            ? onOpenHumanOverview
                            : onGenerateHuman}
                        primaryDisabled={overview.litho.human_docs_complete
                            ? !onOpenHumanOverview
                            : lithoBusy || !acpOk || !onGenerateHuman}
                        secondaryLabel={overview.litho.human_docs_complete
                            ? lithoBusy
                                ? tr("common.generating")
                                : tr("common.regenerate")
                            : undefined}
                        onSecondary={overview.litho.human_docs_complete
                            ? onGenerateHuman
                            : undefined}
                        secondaryDisabled={lithoBusy || !acpOk}
                    />
                    <OverviewKnowledgeCard
                        title={tr("terms.short.agentKnowledge")}
                        subtitle={tr("overview.card.agentSubtitle")}
                        meta={overview.agent_context.ready
                            ? tr("overview.meta.sectionCount", {
                                  count: overview.agent_context.section_count,
                              })
                            : tr("overview.meta.notGenerated")}
                        ready={overview.agent_context.ready}
                        icon="compass"
                        primaryLabel={overview.agent_context.ready
                            ? tr("common.open")
                            : agentContextBusy
                              ? tr("terms.generating")
                              : tr("terms.generate", {
                                    term: tr("terms.agentKnowledge"),
                                })}
                        onPrimary={overview.agent_context.ready
                            ? onOpenArchitectureDoc
                            : onGenerateAgentContext}
                        primaryDisabled={overview.agent_context.ready
                            ? !onOpenArchitectureDoc
                            : agentContextBusy ||
                              !llmReady ||
                              !onGenerateAgentContext}
                        secondaryLabel={overview.agent_context.ready
                            ? agentContextBusy
                                ? tr("common.generating")
                                : tr("common.regenerate")
                            : undefined}
                        onSecondary={overview.agent_context.ready
                            ? onGenerateAgentContext
                            : undefined}
                        secondaryDisabled={agentContextBusy || !llmReady}
                    />
                    {#if structuredAsset}
                        {@const structuredAction =
                            assetPrimaryAction(structuredAsset)}
                        <OverviewKnowledgeCard
                            title={tr("overview.card.structuredTitle")}
                            subtitle={tr("overview.card.structuredSubtitle")}
                            meta={structuredAsset.summary}
                            ready={structuredAsset.ready}
                            icon="list"
                            primaryLabel={structuredAction?.label ??
                                tr("common.open")}
                            onPrimary={structuredAction?.onClick}
                            primaryDisabled={structuredAction?.disabled ??
                                !structuredAction}
                        />
                    {/if}
                </div>

                {#if needsGenerationSetup}
                    <div
                        class="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-tr-border bg-tr-surface px-4 py-3"
                    >
                        <p class="text-xs text-tr-ink-2">
                            {tr("overview.genSetupIntro")}
                            {#if hybridNativeLlm && !llmReady}
                                <span class="text-tr-watch"
                                    >{tr("overview.llmNotConfigured")}</span
                                >
                            {/if}
                            {#if hybridNativeLlm && !llmReady && !acpOk}
                                <span class="text-tr-ink-4"> · </span>
                            {/if}
                            {#if !acpOk}
                                <span class="text-tr-watch"
                                    >{tr("overview.acpNotConnected")}</span
                                >
                            {/if}
                        </p>
                        {#if onOpenSettings}
                            <button
                                type="button"
                                class="shrink-0 text-xs text-tr-accent transition-colors hover:text-tr-accent-hover"
                                onclick={onOpenSettings}
                            >
                                {tr("overview.goToSettings")}
                            </button>
                        {/if}
                    </div>
                {/if}
            </section>

            <!-- Agent 工程环境域 -->
            <section class="space-y-3">
                <div>
                    <h3 class="text-sm font-medium text-tr-ink-2">
                        {tr("terms.agentEnv")}
                    </h3>
                    <p class="mt-0.5 text-xs text-tr-ink-3">
                        {tr("overview.envSubtitle")}
                    </p>
                </div>

                <div
                    class="flex flex-wrap items-center justify-between gap-4 rounded-xl border border-tr-border bg-tr-surface px-5 py-4"
                >
                    <div class="flex min-w-0 items-center gap-3">
                        <span
                            class="text-lg font-semibold [font-variant-numeric:tabular-nums] text-tr-ink"
                        >
                            {overview.agent_env.integrated_count}<span
                                class="text-sm font-normal text-tr-ink-3"
                                >/{overview.agent_env.total_count}</span
                            >
                        </span>
                        <div class="min-w-0">
                            <div class="flex flex-wrap items-center gap-2">
                                <p class="text-sm font-medium text-tr-ink">
                                    {overview.agent_env.summary}
                                </p>
                                <span
                                    class={`rounded-full px-2 py-0.5 text-[10px] font-medium ${
                                        overview.agent_env.ready
                                            ? "bg-tr-good-soft text-tr-good"
                                            : "bg-tr-elevated text-tr-ink-3"
                                    }`}
                                >
                                    {overview.agent_env.ready
                                        ? tr("overview.status.ready")
                                        : tr("overview.status.pendingConfig")}
                                </span>
                            </div>
                            <p class="mt-1 text-xs text-tr-ink-3">
                                {tr("overview.envNote")}
                            </p>
                        </div>
                    </div>
                    {#if onOpenEnv}
                        <button
                            type="button"
                            class="tr-press shrink-0 rounded-xl bg-tr-accent px-4 py-2 text-sm font-medium text-tr-on-accent transition-colors hover:bg-tr-accent-hover"
                            onclick={onOpenEnv}
                        >
                            {overview.agent_env.ready
                                ? tr("overview.manageIntegrations")
                                : tr("overview.actions.envConfigure")}
                        </button>
                    {/if}
                </div>
            </section>

            {#if overview.repo_path || knowledgePath}
                <div
                    class="flex flex-col gap-2 rounded-xl border border-tr-border bg-tr-surface px-4 py-3"
                >
                    {#if overview.repo_path}
                        {@render metaRow(tr("overview.repoPathLabel"), overview.repo_path)}
                    {/if}
                    {#if knowledgePath}
                        {@render metaRow(tr("overview.knowledgePathLabel"), knowledgePath)}
                    {/if}
                </div>
            {/if}
        </div>
    {/if}
</div>

<FreshnessHelpPanel
    open={freshnessHelpOpen}
    {freshness}
    {quickRefreshBusy}
    {onQuickRefresh}
    onclose={() => (freshnessHelpOpen = false)}
/>

<ReadinessHelpPanel
    open={readinessHelpOpen}
    {readyCount}
    {assetTotal}
    rows={assetRows}
    {freshnessBadgeClass}
    {onOpenKnowledge}
    onclose={() => (readinessHelpOpen = false)}
/>
