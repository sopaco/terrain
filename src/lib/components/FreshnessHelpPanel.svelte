<script lang="ts">
    import type { FreshnessSummary } from "../types";
    import { TERMS } from "../terminology";
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

    const scoreBands = [
        {
            min: FRESH_THRESHOLD,
            label: "新鲜",
            tone: "text-tr-good",
            hint: "Ask 可信任预加载的架构概览",
        },
        {
            min: VERIFY_THRESHOLD,
            label: "需核对",
            tone: "text-tr-watch",
            hint: "架构类回答应用源码索引交叉验证",
        },
        {
            min: MACRO_THRESHOLD,
            label: "偏低",
            tone: "text-tr-watch",
            hint: "谨慎引用模块地图与系统边界",
        },
        {
            min: 0,
            label: "过期风险",
            tone: "text-tr-critical",
            hint: "Ask 不预加载宏观架构，以 repomix 为准",
        },
    ];

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
                return "影响较大";
            case "medium":
                return "有影响";
            case "low":
                return "轻微";
            default:
                return "说明";
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
                知识新鲜度说明
            </h2>
            <p class="mt-0.5 text-xs text-tr-ink-3">
                {#if freshness}
                    当前综合分 <span class="font-medium text-tr-ink-2"
                        >{freshness.overall_score}/100</span
                    >
                    · 更新于 {formatComputedAt(freshness.last_computed_at)}
                {:else}
                    打开项目概览后将显示本项目的计算结果
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
                分数含义
            </h3>
            <p class="mt-2 text-sm leading-relaxed text-tr-ink-2">
                新鲜度衡量「知识资产」与「当前代码仓库」的接近程度。分数越高，Ask
                与 Agent 引用架构说明时越可靠。
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
                如何计算
            </h3>
            <p class="mt-2 text-xs leading-relaxed text-tr-ink-2">
                从 100
                分起评，按下列因素扣分（各项有上限）。三层资产分别计分后，<strong
                    class="font-medium text-tr-ink-2">综合分取最低值</strong
                >：
            </p>
            <ul
                class="mt-3 list-inside list-disc space-y-1.5 text-xs text-tr-ink-3"
            >
                <li>
                    <span class="text-tr-ink-2">源码索引</span>（repomix）—
                    对比打包时的 Git baseline 与当前 HEAD
                </li>
                <li>
                    <span class="text-tr-ink-2">{TERMS.agentKnowledge}</span> —
                    按自己的生成 baseline 对比；作为 LLM 派生资产再乘 0.9（故该层上限 90 分，综合分同此上限），且不高于源码索引分数
                </li>
                <li>
                    <span class="text-tr-ink-2">{TERMS.humanKnowledge}</span> — 主要参考项目扫描时间与提交漂移
                </li>
            </ul>
            <div
                class="mt-3 grid gap-2 text-[11px] text-tr-ink-3 sm:grid-cols-2"
            >
                <div
                    class="rounded-lg border border-tr-border bg-tr-page px-3 py-2"
                >
                    每落后 1 个改动源码的提交 · 约 −2 分（上限 40）
                </div>
                <div
                    class="rounded-lg border border-tr-border bg-tr-page px-3 py-2"
                >
                    变更文件占比 · 最多 −30 分
                </div>
                <div
                    class="rounded-lg border border-tr-border bg-tr-page px-3 py-2"
                >
                    距上次同步每多 1 天 · 约 −1 分（上限 5）
                </div>
                <div
                    class="rounded-lg border border-tr-border bg-tr-page px-3 py-2"
                >
                    源码路径有未提交改动 · −5 分
                </div>
            </div>
        </section>

        {#if freshness}
            <section>
                <h3
                    class="text-xs font-semibold uppercase tracking-wider text-tr-ink-3"
                >
                    分层得分
                </h3>
                <div class="mt-2 grid gap-2 sm:grid-cols-3">
                    <div
                        class="rounded-xl border border-tr-border-strong bg-tr-page px-3 py-2.5"
                    >
                        <p class="text-[10px] text-tr-ink-3">源码索引</p>
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
                        <p class="text-[10px] text-tr-ink-3">Agent 上下文</p>
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
                        <p class="text-[10px] text-tr-ink-3">人类文档</p>
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
                        本项目的偏离原因
                    {:else}
                        当前状态
                    {/if}
                </h3>
                {#if negativeFactors.length === 0 && infoFactors.length === 0}
                    <p class="mt-2 text-sm text-tr-ink-3">
                        暂无详细分析。请重新打开概览或运行一次「快速保鲜」以刷新计算。
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
                                            >约 −{factor.points_lost} 分</span
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
                            部分变更文件（共 {freshness.changed_files_count} 个）
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
                <h3 class="text-xs font-semibold text-tr-accent">可以怎么做</h3>
                <ul
                    class="mt-2 list-inside list-disc space-y-1 text-xs leading-relaxed text-tr-ink-2"
                >
                    {#if freshness.working_tree_dirty}
                        <li>
                            提交或暂存<strong>源码</strong
                            >改动，避免「未提交修改」持续扣分（`.terrain/`
                            产出不计入）
                        </li>
                    {/if}
                    {#if freshness.commits_since_baseline > 0 || freshness.changed_files_count > 0}
                        <li>
                            代码已前进时，使用「快速保鲜」更新源码索引与 Agent
                            知识资产（无需重跑 Litho）
                        </li>
                    {/if}
                    {#if freshness.overall_stale}
                        <li>
                            分数低于 80 时，架构类问题请以源码索引 grep
                            结果为准，不要只信 context.md
                        </li>
                    {:else}
                        <li>保持当前节奏即可；大重构后记得再次保鲜</li>
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
                        {quickRefreshBusy ? "保鲜中…" : "立即快速保鲜"}
                    </button>
                {/if}
            </section>
        {/if}
    </div>
</ModalShell>
