<script lang="ts">
  import { tr } from "../i18n";
  import { copyTextToClipboard } from "../clipboard";

  interface Props {
    summary: string;
    detail?: string | null;
    compact?: boolean;
  }

  let { summary, detail = null, compact = false }: Props = $props();

  let expanded = $state(false);
  let copied = $state(false);

  const fullDetail = $derived((detail ?? "").trim());
  const showActions = $derived(
    fullDetail.length > 0 && (fullDetail !== summary.trim() || summary.includes("…")),
  );

  async function copyDetail() {
    const text = fullDetail || summary;
    try {
      await copyTextToClipboard(text);
      copied = true;
      window.setTimeout(() => {
        copied = false;
      }, 2000);
    } catch {
      copied = false;
    }
  }
</script>

<div
  class={`error-notice rounded-lg border border-tr-critical/30 bg-tr-critical-soft text-tr-critical ${
    compact ? "px-2.5 py-2 text-xs" : "px-3 py-2 text-sm"
  }`}
>
  <p class="error-notice-summary m-0 leading-relaxed">{summary}</p>

  {#if showActions}
    <div class="mt-2 flex flex-wrap items-center gap-2">
      <button
        type="button"
        class="tr-press rounded-md border border-tr-critical/25 px-2 py-0.5 text-[11px] font-medium transition-colors hover:bg-tr-critical/10"
        onclick={() => (expanded = !expanded)}
      >
        {expanded ? tr("common.collapse") : tr("misc.errorNotice.showDetails")}
      </button>
      <button
        type="button"
        class="tr-press rounded-md border border-tr-critical/25 px-2 py-0.5 text-[11px] font-medium transition-colors hover:bg-tr-critical/10"
        onclick={() => void copyDetail()}
      >
        {copied ? tr("common.copied") : tr("misc.errorNotice.copyLog")}
      </button>
    </div>
    {#if expanded}
      <pre
        class="error-notice-detail mt-2 max-h-48 overflow-auto rounded-md bg-tr-page/80 p-2 font-mono text-[11px] leading-relaxed text-tr-critical"
      >{fullDetail}</pre>
    {/if}
  {/if}
</div>

<style>
  .error-notice-summary {
    overflow-wrap: anywhere;
    word-break: break-word;
  }

  .error-notice-detail {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    word-break: break-word;
  }
</style>
