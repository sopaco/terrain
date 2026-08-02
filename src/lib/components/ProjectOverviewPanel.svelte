<script lang="ts">
    import { Check, CircleCheck, Copy, FolderOpen } from "@lucide/svelte";
    import type { ProjectOverview, StaleProjectSummary } from "../types";
    import { copyTextToClipboard } from "../clipboard";
    import {
        generateLabel,
        SHORT_TERMS,
        TERMS,
        UI_MESSAGES,
    } from "../terminology";
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
        staleProjects?: StaleProjectSummary[];
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
        staleProjects = [],
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
        onSaveProjectRemark,
    }: Props = $props();

    let freshnessHelpOpen = $state(false);
    let readinessHelpOpen = $state(false);
    let remarkEditing = $state(false);
    let remarkDraft = $state("");
    let remarkSaving = $state(false);
    let copiedPath = $state<string | null>(null);

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
        if (needsLlm) parts.push(`LLM（${TERMS.agentKnowledge}）`);
        if (needsAcp) parts.push(`ACP（${TERMS.humanKnowledge}）`);
        return `部分步骤需要配置：${parts.join("、")}，可在设置中完成后再试`;
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
                `新鲜度 ${freshness.overall_score}/100`,
            ];
            if (freshness.commits_since_baseline > 0) {
                driftParts.push(
                    `落后 ${freshness.commits_since_baseline} 个提交`,
                );
            }
            if (freshness.changed_files_count > 0) {
                driftParts.push(
                    `${freshness.changed_files_count} 个文件已变更`,
                );
            }
            if (freshness.working_tree_dirty) {
                driftParts.push("工作区有未提交修改");
            }
            items.push({
                id: "stale",
                priority: 1,
                accent: "rose",
                title: "知识可能已过期",
                detail: `${driftParts.join(" · ")}。过期架构知识可能误导 Agent 问答。`,
                hint: "建议运行「快速保鲜」更新源码索引与 Agent 知识资产（跳过 Litho）。",
                actionLabel: "快速保鲜",
                busyLabel: "保鲜中…",
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
                title: "部分知识资产尚未就绪",
                detail: `当前 ${readyCount}/${assetTotal} 项就绪。可一键完成扫描、源码索引、${TERMS.agentKnowledge} 与 ${TERMS.humanKnowledge}。`,
                hint: initHint ?? undefined,
                actionLabel: "一键初始化",
                busyLabel: initProgress ?? "初始化中…",
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
                title: `${TERMS.agentEnv}尚未配置`,
                detail: `为 Coding Agent 集成 Skills、工具链与 AGENTS.md。当前 ${overview.agent_env.summary}。`,
                actionLabel: "前往配置",
                onAction: onOpenEnv,
            });
        }

        return items;
    });

    const staleActionItems = $derived.by((): OverviewActionItem[] =>
        staleProjects.map((stale) => ({
            id: `stale-${stale.slug}`,
            priority: 2,
            accent: "amber" as const,
            title: stale.slug,
            detail: "仓库 `.terrain` 已缺失或损坏，可一键重新扫描并生成知识资产。",
            hint: initHint ?? undefined,
            actionLabel: "重新初始化",
            busyLabel: initProgress ?? "初始化中…",
            onAction: onInitializeProject
                ? () => onInitializeProject(stale.repo_path, stale.slug)
                : undefined,
            disabled: initBusy || !onInitializeProject,
            busy: initBusy,
        })),
    );

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
            return `${o.litho.human_doc_count} 篇文档`;
        }
        if (o.litho.has_human_docs) {
            return `${o.litho.human_doc_count} 篇（未完成）`;
        }
        if (o.litho.has_research_artifacts) {
            return "研究稿已就绪，待编排";
        }
        return "尚未生成";
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
                return { label: "浏览", onClick: onOpenHumanOverview };
            }
            if (onGenerateHuman) {
                return {
                    label: generateLabel(TERMS.humanKnowledge, lithoBusy),
                    onClick: onGenerateHuman,
                    disabled: lithoBusy || !acpOk,
                };
            }
        } else if (asset.track === "agent_context") {
            if (asset.ready && onOpenArchitectureDoc) {
                return { label: "浏览", onClick: onOpenArchitectureDoc };
            }
            if (onGenerateAgentContext) {
                return {
                    label: asset.ready
                        ? agentContextBusy
                            ? "生成中…"
                            : "重新生成"
                        : generateLabel(TERMS.agentKnowledge, false),
                    onClick: onGenerateAgentContext,
                    disabled: agentContextBusy || !llmReady,
                };
            }
        } else if (asset.track === "agent_pack" && onRepack) {
            return {
                label: repackBusy ? UI_MESSAGES.repacking : UI_MESSAGES.repack,
                onClick: onRepack,
                disabled: repackBusy,
            };
        } else if (asset.track === "structured") {
            if (asset.ready && onOpenStructured) {
                return { label: "浏览", onClick: onOpenStructured };
            }
            return { label: "扫描项目", onClick: onOpenKnowledge };
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
            title="复制"
            aria-label={`复制${label}`}
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
                title="在 Finder 中打开"
                aria-label={`打开${label}`}
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
            <span>加载项目概览…</span>
        </div>
    {:else if !overview}
        <div
            class="mx-auto flex w-full max-w-3xl flex-1 flex-col justify-center gap-6 px-6 py-10"
        >
            <section
                class="rounded-2xl border border-tr-border bg-tr-surface px-6 py-8"
            >
                <h2 class="text-xl font-semibold text-tr-ink">
                    欢迎使用 Terrain
                </h2>
                <p class="mt-2 text-sm leading-relaxed text-tr-ink-2">
                    添加本地仓库后将自动完成索引与知识资产生成，可在本页查看状态并进入知识库阅读。
                </p>
                {#if staleProjects.length === 0}
                    <p class="mt-4 text-xs text-tr-ink-3">
                        通过顶部项目选择器添加本地仓库；若索引失败，添加后可在本页初始化。
                    </p>
                {/if}
            </section>

            {#if staleProjects.length > 0}
                <section class="space-y-3">
                    <h3 class="text-sm font-medium text-tr-ink-2">
                        检测到知识库数据丢失
                    </h3>
                    <OverviewActionBanner
                        items={staleActionItems}
                        progressNote={initBusy ? initProgress : null}
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
                                placeholder="填写项目备注，将保存至 .terrain/project-note.md"
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
                                    {remarkSaving ? "保存中…" : "保存备注"}
                                </button>
                                <button
                                    type="button"
                                    class="tr-press rounded-lg border border-tr-border-strong px-3 py-1.5 text-xs text-tr-ink-2 transition-colors hover:bg-tr-elevated"
                                    disabled={remarkSaving}
                                    onclick={cancelRemarkEdit}
                                >
                                    取消
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
                                    添加项目备注，便于团队识别此仓库
                                </p>
                            {/if}
                            {#if onSaveProjectRemark}
                                <button
                                    type="button"
                                    class="shrink-0 text-[11px] text-tr-ink-3 transition-colors hover:text-tr-accent"
                                    onclick={startRemarkEdit}
                                >
                                    编辑
                                </button>
                            {/if}
                        </div>
                    {/if}

                    <p
                        class="mt-2 flex flex-wrap items-center gap-1.5 text-xs text-tr-ink-3 [font-variant-numeric:tabular-nums]"
                    >
                        <span
                            >最后同步 {formatSyncedAt(overview.synced_at)}</span
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
                            <span class="text-tr-watch">工作区有未提交修改</span
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
                            打开仓库
                        </button>
                    {/if}
                    <button
                        type="button"
                        class="tr-press rounded-xl bg-tr-accent px-4 py-2 text-sm font-medium text-tr-on-accent transition-colors hover:bg-tr-accent-hover"
                        onclick={onOpenAsk}
                    >
                        提问 Ask
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
                            <span class="text-xs text-tr-ink-2">结构就绪度</span
                            >
                            <HelpButton
                                onclick={() => (readinessHelpOpen = true)}
                                title="查看各项知识资产就绪情况"
                                ariaLabel="就绪度说明"
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
                            <span class="text-xs text-tr-ink-2">知识新鲜度</span
                            >
                            <HelpButton
                                onclick={() => (freshnessHelpOpen = true)}
                                title="了解新鲜度如何计算及本项目的偏离原因"
                                ariaLabel="知识新鲜度说明"
                                size={14}
                            />
                        </div>
                        {#if freshnessLoading && !freshness}
                            <span class="text-xs text-tr-ink-3">计算中…</span>
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
                            class="mt-3 h-1.5 animate-pulse rounded-full bg-tr-elevated"
                            role="status"
                            aria-live="polite"
                        ></div>
                    {:else if freshnessScore != null}
                        <div
                            class="mt-3 h-1.5 overflow-hidden rounded-full bg-tr-elevated"
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
                                更新中…
                            {:else if freshness?.is_git_repo && freshness.current_git_head}
                                基于 Git 提交对比 · HEAD {freshness.current_git_head}
                                {#if freshness.working_tree_dirty}
                                    · 工作区有未提交修改
                                {/if}
                            {:else if freshness && !freshness.is_git_repo}
                                未检测到 Git，分数按知识资产同步时间估算
                            {/if}
                        </p>
                    {/if}
                </div>
            </div>

            <!-- 知识资产域 -->
            <section class="space-y-3">
                <div>
                    <h3 class="text-sm font-medium text-tr-ink-2">知识资产</h3>
                    <p class="mt-0.5 text-xs text-tr-ink-3">
                        仓库内 <code class="text-tr-ink-2">.terrain/</code> 的三种读取方式，按用途区分
                    </p>
                </div>

                <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                    <OverviewKnowledgeCard
                        title={SHORT_TERMS.agentKnowledge}
                        subtitle="模块地图、架构与流程等，供 Agent 与问答使用"
                        meta={overview.agent_context.ready
                            ? `${overview.agent_context.section_count} 个章节`
                            : "尚未生成"}
                        ready={overview.agent_context.ready}
                        icon="compass"
                        primaryLabel={overview.agent_context.ready
                            ? "打开"
                            : generateLabel(
                                  TERMS.agentKnowledge,
                                  agentContextBusy,
                              )}
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
                                ? "生成中…"
                                : "重新生成"
                            : undefined}
                        onSecondary={overview.agent_context.ready
                            ? onGenerateAgentContext
                            : undefined}
                        secondaryDisabled={agentContextBusy || !llmReady}
                    />
                    <OverviewKnowledgeCard
                        title={SHORT_TERMS.humanKnowledge}
                        subtitle="Litho C4 文档，从 1.概述 开始阅读"
                        meta={humanKnowledgeMeta(overview)}
                        ready={overview.litho.human_docs_complete}
                        icon="book"
                        primaryLabel={overview.litho.human_docs_complete
                            ? "打开"
                            : generateLabel(TERMS.humanKnowledge, lithoBusy)}
                        onPrimary={overview.litho.human_docs_complete
                            ? onOpenHumanOverview
                            : onGenerateHuman}
                        primaryDisabled={overview.litho.human_docs_complete
                            ? !onOpenHumanOverview
                            : lithoBusy || !acpOk || !onGenerateHuman}
                        secondaryLabel={overview.litho.human_docs_complete
                            ? lithoBusy
                                ? "生成中…"
                                : "重新生成"
                            : undefined}
                        onSecondary={overview.litho.human_docs_complete
                            ? onGenerateHuman
                            : undefined}
                        secondaryDisabled={lithoBusy || !acpOk}
                    />
                    {#if structuredAsset}
                        {@const structuredAction =
                            assetPrimaryAction(structuredAsset)}
                        <OverviewKnowledgeCard
                            title="结构化条目"
                            subtitle="terrain-meta.json 派生的元数据，供工具消费"
                            meta={structuredAsset.summary}
                            ready={structuredAsset.ready}
                            icon="list"
                            primaryLabel={structuredAction?.label ?? "打开"}
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
                            生成知识资产需要 Terrain 生成能力：
                            {#if hybridNativeLlm && !llmReady}
                                <span class="text-tr-watch">LLM 未配置</span>
                            {/if}
                            {#if hybridNativeLlm && !llmReady && !acpOk}
                                <span class="text-tr-ink-4"> · </span>
                            {/if}
                            {#if !acpOk}
                                <span class="text-tr-watch">ACP 未连接</span>
                            {/if}
                        </p>
                        {#if onOpenSettings}
                            <button
                                type="button"
                                class="shrink-0 text-xs text-tr-accent transition-colors hover:text-tr-accent-hover"
                                onclick={onOpenSettings}
                            >
                                前往设置
                            </button>
                        {/if}
                    </div>
                {/if}
            </section>

            <!-- Agent 工程环境域 -->
            <section class="space-y-3">
                <div>
                    <h3 class="text-sm font-medium text-tr-ink-2">
                        {TERMS.agentEnv}
                    </h3>
                    <p class="mt-0.5 text-xs text-tr-ink-3">
                        Skills、工具链与 AGENTS.md，供 Coding Agent 在仓库中协作
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
                                        ? "已集成"
                                        : "待配置"}
                                </span>
                            </div>
                            <p class="mt-1 text-xs text-tr-ink-3">
                                与知识资产生成无关，用于优化外部 Coding Agent
                                在本仓库的工作体验。
                            </p>
                        </div>
                    </div>
                    {#if onOpenEnv}
                        <button
                            type="button"
                            class="tr-press shrink-0 rounded-xl bg-tr-accent px-4 py-2 text-sm font-medium text-tr-on-accent transition-colors hover:bg-tr-accent-hover"
                            onclick={onOpenEnv}
                        >
                            {overview.agent_env.ready ? "管理集成" : "前往配置"}
                        </button>
                    {/if}
                </div>
            </section>

            {#if overview.repo_path || knowledgePath}
                <div
                    class="flex flex-col gap-2 rounded-xl border border-tr-border bg-tr-surface px-4 py-3"
                >
                    {#if overview.repo_path}
                        {@render metaRow("仓库路径", overview.repo_path)}
                    {/if}
                    {#if knowledgePath}
                        {@render metaRow("知识库路径", knowledgePath)}
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
