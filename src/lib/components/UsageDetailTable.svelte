<script lang="ts">
  import type { UsagePeriodEntry } from "../types";

  interface Props {
    title: string;
    labelColumn: string;
    rows: UsagePeriodEntry[];
    rowLabel: (row: UsagePeriodEntry) => string;
    emptyLabel?: string;
  }

  let {
    title,
    labelColumn,
    rows,
    rowLabel,
    emptyLabel = "暂无明细数据",
  }: Props = $props();

  function formatTokens(n: number): string {
    return n.toLocaleString();
  }

  function formatCost(usd: number): string {
    if (usd <= 0) return "$0.00";
    return `$${usd.toFixed(2)}`;
  }

  function agentsLabel(row: UsagePeriodEntry): string {
    if (row.agents.length > 0) return row.agents.join(", ");
    return row.agent ?? "—";
  }
</script>

<section>
  <h3 class="mb-2 text-xs font-medium uppercase tracking-wide text-white/40">{title}</h3>
  {#if rows.length === 0}
    <p class="py-6 text-center text-sm text-white/45">{emptyLabel}</p>
  {:else}
    <div class="overflow-x-auto rounded-xl border border-white/10">
      <table class="w-full min-w-[36rem] text-left text-xs">
        <thead class="border-b border-white/10 bg-white/[0.03] text-white/45">
          <tr>
            <th class="px-3 py-2 font-medium">{labelColumn}</th>
            <th class="px-3 py-2 font-medium">Agent</th>
            <th class="px-3 py-2 font-medium text-right">输入</th>
            <th class="px-3 py-2 font-medium text-right">输出</th>
            <th class="px-3 py-2 font-medium text-right">Cache</th>
            <th class="px-3 py-2 font-medium text-right">成本</th>
          </tr>
        </thead>
        <tbody>
          {#each rows as row}
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
</section>
