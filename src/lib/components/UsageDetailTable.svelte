<script lang="ts">
  import { FolderOpen } from "@lucide/svelte";
  import type { UsagePeriodEntry } from "../types";

  interface Props {
    title: string;
    labelColumn: string;
    rows: UsagePeriodEntry[];
    rowLabel: (row: UsagePeriodEntry) => string;
    emptyLabel?: string;
    loading?: boolean;
    loadingLabel?: string;
    rowOpenPath?: (row: UsagePeriodEntry) => string | null | undefined;
    onRowOpen?: (path: string) => void | Promise<void>;
  }

  let {
    title,
    labelColumn,
    rows,
    rowLabel,
    emptyLabel = "暂无明细数据",
    loading = false,
    loadingLabel = "正在加载…",
    rowOpenPath,
    onRowOpen,
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

  function openPath(path: string) {
    void onRowOpen?.(path);
  }
</script>

<section>
  <h3 class="mb-2 text-xs font-medium uppercase tracking-wide text-tr-ink-3">{title}</h3>
  {#if loading && rows.length === 0}
    <div class="flex items-center justify-center gap-2 rounded-xl border border-tr-border-strong py-8 text-sm text-tr-ink-3">
      <span
        class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-tr-accent border-t-transparent"
      ></span>
      {loadingLabel}
    </div>
  {:else if rows.length === 0}
    <p class="py-6 text-center text-sm text-tr-ink-3">{emptyLabel}</p>
  {:else}
    <div class="overflow-x-auto rounded-xl border border-tr-border-strong">
      <table class="w-full min-w-[36rem] text-left text-xs">
        <thead class="border-b border-tr-border-strong bg-tr-elevated text-tr-ink-3">
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
            {@const path = rowOpenPath?.(row) ?? null}
            <tr class="border-b border-tr-border last:border-0 hover:bg-tr-elevated">
              <td class="px-3 py-2 text-tr-ink-2">
                {#if path && onRowOpen}
                  <button
                    type="button"
                    class="tr-press group inline-flex max-w-full items-center gap-1.5 rounded-md px-1 py-0.5 text-left transition-colors hover:bg-tr-page hover:text-tr-accent"
                    title="在文件管理器中查看本地记录"
                    aria-label={`在文件管理器中打开 ${rowLabel(row)}`}
                    onclick={() => openPath(path)}
                  >
                    <span class="truncate underline decoration-tr-border-strong decoration-dotted underline-offset-2 group-hover:decoration-tr-accent/60">
                      {rowLabel(row)}
                    </span>
                    <FolderOpen
                      size={12}
                      strokeWidth={2}
                      class="shrink-0 text-tr-ink-3 transition-colors group-hover:text-tr-accent"
                      aria-hidden="true"
                    />
                  </button>
                {:else}
                  {rowLabel(row)}
                {/if}
              </td>
              <td class="px-3 py-2 text-tr-ink-2">{agentsLabel(row)}</td>
              <td class="px-3 py-2 text-right text-tr-ink-2">{formatTokens(row.input_tokens)}</td>
              <td class="px-3 py-2 text-right text-tr-ink-2">{formatTokens(row.output_tokens)}</td>
              <td class="px-3 py-2 text-right text-tr-ink-3">
                {formatTokens(row.cache_creation_tokens + row.cache_read_tokens)}
              </td>
              <td class="px-3 py-2 text-right text-tr-accent">{formatCost(row.total_cost_usd)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</section>
