<script lang="ts">
  import type { AssetTrackHealth } from "../types";
  import { TERMS } from "../terminology";
  import ModalShell from "./ModalShell.svelte";

  interface AssetAction {
    label: string;
    onClick?: () => void;
    disabled?: boolean;
  }

  interface AssetRow {
    asset: AssetTrackHealth;
    action: AssetAction | null;
  }

  interface Props {
    open: boolean;
    readyCount: number;
    assetTotal: number;
    rows: AssetRow[];
    onclose: () => void;
    onOpenKnowledge: () => void;
    freshnessBadgeClass: (score: number, stale?: boolean | null) => string;
  }

  let {
    open,
    readyCount,
    assetTotal,
    rows,
    onclose,
    onOpenKnowledge,
    freshnessBadgeClass,
  }: Props = $props();
</script>

<ModalShell
  {open}
  {onclose}
  ariaLabelledby="readiness-help-title"
  dialogClass="max-w-[min(92vw,560px)] max-h-[min(85vh,680px)]"
>
  <header class="flex shrink-0 items-start justify-between gap-3 border-b border-white/10 px-5 py-4">
      <div class="min-w-0">
        <h2 id="readiness-help-title" class="text-base font-semibold text-white/95">知识资产就绪说明</h2>
        <p class="mt-0.5 text-xs text-white/45">
          当前 <span class="font-medium text-white/80">{readyCount}/{assetTotal}</span> 项就绪
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

    <div class="flex-1 space-y-4 overflow-y-auto px-5 py-4">
      <section>
        <p class="text-sm leading-relaxed text-white/60">
          就绪度衡量仓库内 <code class="text-white/70">.mind-mesh/</code> 各项知识资产是否已生成并可供 Agent 与人类阅读使用，包括源码索引、架构上下文、人类文档等。
        </p>
      </section>

      <section class="space-y-2">
        <div class="flex items-center justify-between gap-2">
          <h3 class="text-xs font-medium text-white/50">各项资产</h3>
          <button
            type="button"
            class="text-xs text-indigo-300/90 hover:text-indigo-200"
            onclick={onOpenKnowledge}
          >
            进入{TERMS.knowledgeTab}
          </button>
        </div>
        {#each rows as row (row.asset.label)}
          <div
            class={`flex items-center justify-between gap-3 rounded-xl border px-4 py-3 ${
              row.asset.ready ? "border-white/8 bg-white/[0.02]" : "border-white/8 bg-transparent"
            }`}
          >
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <p class="text-sm font-medium text-white/85">{row.asset.label}</p>
                <span
                  class={`rounded-full px-2 py-0.5 text-[10px] font-medium ${
                    row.asset.ready
                      ? "bg-emerald-500/15 text-emerald-200"
                      : "bg-amber-500/10 text-amber-200/90"
                  }`}
                >
                  {row.asset.ready ? "就绪" : "待生成"}
                </span>
                {#if row.asset.freshness_score != null}
                  <span
                    class={`rounded-full px-2 py-0.5 text-[10px] font-medium ${freshnessBadgeClass(row.asset.freshness_score, row.asset.stale)}`}
                  >
                    新鲜度 {row.asset.freshness_score}
                  </span>
                {/if}
              </div>
              <p class="mt-0.5 text-xs text-white/45">{row.asset.summary}</p>
            </div>
            {#if row.action?.onClick}
              <button
                type="button"
                class="shrink-0 rounded-lg border border-white/10 px-2.5 py-1 text-[11px] text-indigo-300/90 hover:bg-white/5 hover:text-indigo-200 disabled:opacity-50"
                disabled={row.action.disabled}
                onclick={row.action.onClick}
              >
                {row.action.label}
              </button>
            {/if}
          </div>
        {/each}
      </section>
    </div>
</ModalShell>
