<script lang="ts">
  import type { AssetTrackHealth } from "../types";
  import { tr } from "../i18n";
  import CloseButton from "./icons/CloseButton.svelte";
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
  <header class="flex shrink-0 items-start justify-between gap-3 border-b border-tr-border-strong px-5 py-4">
      <div class="min-w-0">
        <h2 id="readiness-help-title" class="text-base font-semibold text-tr-ink">{tr("overview.readinessHelp.title")}</h2>
        <p class="mt-0.5 text-xs text-tr-ink-3">
          {tr("overview.readinessHelp.currentPrefix")} <span class="font-medium text-tr-ink-2">{readyCount}/{assetTotal}</span> {tr("overview.readinessHelp.currentSuffix")}
        </p>
      </div>
      <CloseButton onclick={onclose} class="py-1 text-sm" />
    </header>

    <div class="flex-1 space-y-4 overflow-y-auto px-5 py-4">
      <section>
        <p class="text-sm leading-relaxed text-tr-ink-2">
          {tr("overview.readinessHelp.body1")} <code class="text-tr-ink-2">.terrain/</code> {tr("overview.readinessHelp.body2")}
        </p>
      </section>

      <section class="space-y-2">
        <div class="flex items-center justify-between gap-2">
          <h3 class="text-xs font-medium text-tr-ink-3">{tr("overview.readinessHelp.assetsHeading")}</h3>
          <button
            type="button"
            class="text-xs text-tr-accent transition-colors hover:text-tr-accent-hover"
            onclick={onOpenKnowledge}
          >
            {tr("overview.readinessHelp.enterKnowledge", {
              knowledgeTab: tr("terms.knowledgeTab"),
            })}
          </button>
        </div>
        {#each rows as row (row.asset.label)}
          <div
            class={`flex items-center justify-between gap-3 rounded-xl border px-4 py-3 ${
              row.asset.ready ? "border-tr-border bg-tr-elevated" : "border-tr-border bg-transparent"
            }`}
          >
            <div class="min-w-0 flex-1">
              <div class="flex flex-wrap items-center gap-2">
                <p class="text-sm font-medium text-tr-ink-2">{row.asset.label}</p>
                <span
                  class={`rounded-full px-2 py-0.5 text-[10px] font-medium ${
                    row.asset.ready
                      ? "bg-tr-good-soft text-tr-good"
                      : "bg-tr-watch-soft text-tr-watch"
                  }`}
                >
                  {row.asset.ready
                    ? tr("overview.status.ready")
                    : tr("overview.status.pendingGenerate")}
                </span>
                {#if row.asset.freshness_score != null}
                  <span
                    class={`rounded-full px-2 py-0.5 text-[10px] font-medium ${freshnessBadgeClass(row.asset.freshness_score, row.asset.stale)}`}
                  >
                    {tr("freshness.scoreShort", {
                      score: row.asset.freshness_score,
                    })}
                  </span>
                {/if}
              </div>
              <p class="mt-0.5 text-xs text-tr-ink-3">{row.asset.summary}</p>
            </div>
            {#if row.action?.onClick}
              <button
                type="button"
                class="tr-press shrink-0 rounded-lg border border-tr-border-strong px-2.5 py-1 text-[11px] text-tr-accent transition-colors hover:bg-tr-elevated hover:text-tr-accent-hover disabled:opacity-50"
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
