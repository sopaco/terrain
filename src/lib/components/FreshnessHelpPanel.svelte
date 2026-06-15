<script lang="ts">
  import type { FreshnessSummary } from "../types";
  import { TERMS } from "../terminology";

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
    { min: FRESH_THRESHOLD, label: "新鲜", tone: "text-emerald-200", hint: "Ask 可信任预加载的架构概览" },
    { min: VERIFY_THRESHOLD, label: "需核对", tone: "text-amber-200", hint: "架构类回答应用源码索引交叉验证" },
    { min: MACRO_THRESHOLD, label: "偏低", tone: "text-amber-200", hint: "谨慎引用模块地图与系统边界" },
    { min: 0, label: "过期风险", tone: "text-rose-200", hint: "Ask 不预加载宏观架构，以 repomix 为准" },
  ];

  function severityStyle(severity: string): string {
    switch (severity) {
      case "high":
        return "border-rose-500/25 bg-rose-500/[0.06]";
      case "medium":
        return "border-amber-500/25 bg-amber-500/[0.06]";
      case "low":
        return "border-white/10 bg-white/[0.02]";
      default:
        return "border-indigo-500/20 bg-indigo-500/[0.04]";
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

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-[300] bg-black/50 backdrop-blur-sm"
    onclick={onclose}
    role="presentation"
  ></div>
  <div
    class="fixed left-1/2 top-1/2 z-[301] flex max-h-[min(85vh,680px)] w-[min(92vw,560px)] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-2xl border border-white/10 bg-[#1a1e26] shadow-2xl"
    role="dialog"
    aria-labelledby="freshness-help-title"
    aria-modal="true"
  >
    <header class="flex shrink-0 items-start justify-between gap-3 border-b border-white/10 px-5 py-4">
      <div class="min-w-0">
        <h2 id="freshness-help-title" class="text-base font-semibold text-white/95">知识新鲜度说明</h2>
        <p class="mt-0.5 text-xs text-white/45">
          {#if freshness}
            当前综合分 <span class="font-medium text-white/80">{freshness.overall_score}/100</span>
            · 更新于 {formatComputedAt(freshness.last_computed_at)}
          {:else}
            打开项目概览后将显示本项目的计算结果
          {/if}
        </p>
      </div>
      <button
        type="button"
        class="shrink-0 rounded-lg border border-white/10 px-2.5 py-1 text-sm text-white/60 hover:bg-white/5"
        onclick={onclose}
        aria-label="关闭"
      >
        ✕
      </button>
    </header>

    <div class="flex-1 space-y-5 overflow-y-auto px-5 py-4">
      <section>
        <h3 class="text-xs font-semibold uppercase tracking-wider text-white/40">分数含义</h3>
        <p class="mt-2 text-sm leading-relaxed text-white/60">
          新鲜度衡量「知识资产」与「当前代码仓库」的接近程度。分数越高，Ask 与 Agent 引用架构说明时越可靠。
        </p>
        <ul class="mt-3 space-y-2">
          {#each scoreBands as band}
            <li class="flex items-start gap-2 text-xs">
              <span class={`mt-0.5 w-14 shrink-0 font-medium ${band.tone}`}>≥{band.min}</span>
              <span class="text-white/45">{band.hint}</span>
            </li>
          {/each}
        </ul>
      </section>

      <section class="rounded-xl border border-white/8 bg-white/[0.02] px-4 py-3">
        <h3 class="text-xs font-semibold uppercase tracking-wider text-white/40">如何计算</h3>
        <p class="mt-2 text-xs leading-relaxed text-white/55">
          从 100 分起评，按下列因素扣分（各项有上限）。三层资产分别计分后，<strong class="font-medium text-white/75">综合分取最低值</strong>：
        </p>
        <ul class="mt-3 list-inside list-disc space-y-1.5 text-xs text-white/50">
          <li><span class="text-white/70">源码索引</span>（repomix）— 对比打包时的 Git baseline 与当前 HEAD</li>
          <li><span class="text-white/70">{TERMS.agentKnowledge}</span> — 同上，且不超过源码索引分数的 90%</li>
          <li><span class="text-white/70">{TERMS.humanKnowledge}</span> — 主要参考项目扫描时间与提交漂移</li>
        </ul>
        <div class="mt-3 grid gap-2 text-[11px] text-white/45 sm:grid-cols-2">
          <div class="rounded-lg border border-white/8 bg-black/20 px-3 py-2">
            每落后 1 个提交 · 约 −2 分（上限 40）
          </div>
          <div class="rounded-lg border border-white/8 bg-black/20 px-3 py-2">
            变更文件占比 · 最多 −30 分
          </div>
          <div class="rounded-lg border border-white/8 bg-black/20 px-3 py-2">
            距上次同步每多 1 天 · 约 −2 分（上限 20）
          </div>
          <div class="rounded-lg border border-white/8 bg-black/20 px-3 py-2">
            源码路径有未提交改动 · −5 分（不含 `.mind-mesh/` 知识产出）
          </div>
        </div>
      </section>

      {#if freshness}
        <section>
          <h3 class="text-xs font-semibold uppercase tracking-wider text-white/40">分层得分</h3>
          <div class="mt-2 grid gap-2 sm:grid-cols-3">
            <div class="rounded-xl border border-white/10 bg-black/20 px-3 py-2.5">
              <p class="text-[10px] text-white/35">源码索引</p>
              <p class="text-lg font-semibold text-white/90">{freshness.agent_pack_score}</p>
              {#if freshness.pack_baseline_short}
                <p class="text-[10px] text-white/30">baseline {freshness.pack_baseline_short}</p>
              {/if}
            </div>
            <div class="rounded-xl border border-white/10 bg-black/20 px-3 py-2.5">
              <p class="text-[10px] text-white/35">Agent 上下文</p>
              <p class="text-lg font-semibold text-white/90">{freshness.agent_context_score}</p>
              {#if freshness.context_baseline_short}
                <p class="text-[10px] text-white/30">baseline {freshness.context_baseline_short}</p>
              {/if}
            </div>
            <div class="rounded-xl border border-white/10 bg-black/20 px-3 py-2.5">
              <p class="text-[10px] text-white/35">人类文档</p>
              <p class="text-lg font-semibold text-white/90">{freshness.human_docs_score}</p>
            </div>
          </div>
        </section>

        <section>
          <h3 class="text-xs font-semibold uppercase tracking-wider text-white/40">
            {#if negativeFactors.length > 0}
              本项目的偏离原因
            {:else}
              当前状态
            {/if}
          </h3>
          {#if negativeFactors.length === 0 && infoFactors.length === 0}
            <p class="mt-2 text-sm text-white/50">暂无详细分析。请重新打开概览或运行一次「快速保鲜」以刷新计算。</p>
          {:else}
            <ul class="mt-2 space-y-2">
              {#each negativeFactors as factor}
                <li class={`rounded-xl border px-3.5 py-3 ${severityStyle(factor.severity)}`}>
                  <div class="flex flex-wrap items-center gap-2">
                    <span class="text-sm font-medium text-white/90">{factor.title}</span>
                    <span class="rounded-full bg-black/25 px-1.5 py-0.5 text-[10px] text-white/45">
                      {severityLabel(factor.severity)}
                    </span>
                    {#if factor.points_lost != null && factor.points_lost > 0}
                      <span class="text-[10px] text-rose-200/80">约 −{factor.points_lost} 分</span>
                    {/if}
                  </div>
                  <p class="mt-1.5 text-xs leading-relaxed text-white/55">{factor.detail}</p>
                </li>
              {/each}
            </ul>
          {/if}

          {#if freshness.sample_changed_files?.length}
            <div class="mt-3 rounded-xl border border-white/8 bg-black/20 px-3 py-2.5">
              <p class="text-[10px] font-semibold uppercase tracking-wider text-white/35">
                部分变更文件（共 {freshness.changed_files_count} 个）
              </p>
              <ul class="mt-2 max-h-28 space-y-0.5 overflow-y-auto font-mono text-[11px] text-white/45">
                {#each freshness.sample_changed_files as path}
                  <li class="truncate" title={path}>{path}</li>
                {/each}
              </ul>
            </div>
          {/if}

          {#if infoFactors.length > 0}
            <ul class="mt-3 space-y-2">
              {#each infoFactors as factor}
                <li class={`rounded-xl border px-3.5 py-2.5 ${severityStyle(factor.severity)}`}>
                  <p class="text-xs font-medium text-indigo-200/90">{factor.title}</p>
                  <p class="mt-1 text-xs leading-relaxed text-white/50">{factor.detail}</p>
                </li>
              {/each}
            </ul>
          {/if}
        </section>

        <section class="rounded-xl border border-indigo-500/20 bg-indigo-500/[0.05] px-4 py-3">
          <h3 class="text-xs font-semibold text-indigo-200/90">可以怎么做</h3>
          <ul class="mt-2 list-inside list-disc space-y-1 text-xs leading-relaxed text-white/55">
            {#if freshness.working_tree_dirty}
              <li>提交或暂存<strong>源码</strong>改动，避免「未提交修改」持续扣分（`.mind-mesh/` 产出不计入）</li>
            {/if}
            {#if freshness.commits_since_baseline > 0 || freshness.changed_files_count > 0}
              <li>代码已前进时，使用「快速保鲜」更新源码索引与 Agent 知识资产（无需重跑 Litho）</li>
            {/if}
            {#if freshness.overall_stale}
              <li>分数低于 80 时，架构类问题请以源码索引 grep 结果为准，不要只信 context.md</li>
            {:else}
              <li>保持当前节奏即可；大重构后记得再次保鲜</li>
            {/if}
          </ul>
          {#if onQuickRefresh}
            <button
              type="button"
              class="mt-3 rounded-lg bg-indigo-600 px-3 py-1.5 text-xs font-medium hover:bg-indigo-500 disabled:opacity-50"
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
  </div>
{/if}
