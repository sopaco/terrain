<script lang="ts">
  export type StatusKind = "idle" | "loading" | "progress" | "success" | "error";

  import { Check, CircleAlert } from "@lucide/svelte";
  import { tr } from "../i18n";
  import { copyTextToClipboard } from "../clipboard";

  interface Props {
    message: string;
    kind?: StatusKind;
    detail?: string | null;
  }

  let { message, kind = "idle", detail = null }: Props = $props();

  let expanded = $state(false);
  let copied = $state(false);

  const chipStyles: Record<StatusKind, string> = {
    idle: "border-tr-border-strong bg-tr-elevated text-tr-ink-2",
    loading: "border-tr-accent-soft-strong bg-tr-accent-soft text-tr-accent",
    progress: "border-tr-watch/30 bg-tr-watch-soft text-tr-watch",
    success: "border-tr-good/35 bg-tr-good-soft text-tr-good",
    error: "border-tr-critical/30 bg-tr-critical-soft text-tr-critical",
  };

  const labels = $derived({
    idle: tr("terms.statusChip.idle"),
    loading: tr("terms.statusChip.loading"),
    progress: tr("terms.statusChip.progress"),
    success: tr("terms.statusChip.success"),
    error: tr("terms.statusChip.error"),
  } as Record<StatusKind, string>);

  const fullDetail = $derived((detail ?? "").trim());
  const showErrorActions = $derived(
    kind === "error" &&
      fullDetail.length > 0 &&
      (fullDetail !== message.trim() || message.includes("…")),
  );

  async function copyDetail() {
    const text = fullDetail || message;
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
  role="status"
  class={`status-banner flex min-w-0 flex-col gap-1.5 rounded-lg border px-2.5 py-1.5 text-xs ${chipStyles[kind]}`}
>
  <div class="flex min-w-0 items-start gap-2">
    <div class="flex shrink-0 items-center gap-1.5">
      {#if kind === "loading" || kind === "progress"}
        <span
          class="inline-block h-3 w-3 shrink-0 animate-spin rounded-full border border-current border-t-transparent"
        ></span>
      {:else if kind === "success"}
        <Check size={12} strokeWidth={2.5} class="shrink-0" aria-hidden="true" />
      {:else if kind === "error"}
        <CircleAlert size={12} strokeWidth={2.5} class="shrink-0" aria-hidden="true" />
      {/if}
      <span class="whitespace-nowrap font-medium uppercase tracking-wide opacity-70">{labels[kind]}</span>
    </div>
    <p class="status-banner-message m-0 min-w-0 flex-1 leading-snug">{message}</p>
  </div>

  {#if showErrorActions}
    <div class="flex flex-wrap items-center gap-2 pl-[calc(0.75rem+3.25rem)]">
      <button
        type="button"
        class="tr-press rounded-md border border-current/20 px-2 py-0.5 text-[10px] font-medium opacity-90 transition-colors hover:opacity-100"
        onclick={() => (expanded = !expanded)}
      >
        {expanded ? tr("common.collapse") : tr("misc.errorNotice.showDetails")}
      </button>
      <button
        type="button"
        class="tr-press rounded-md border border-current/20 px-2 py-0.5 text-[10px] font-medium opacity-90 transition-colors hover:opacity-100"
        onclick={() => void copyDetail()}
      >
        {copied ? tr("common.copied") : tr("misc.errorNotice.copyLog")}
      </button>
    </div>
    {#if expanded}
      <pre
        class="status-banner-detail ml-[calc(0.75rem+3.25rem)] max-h-40 overflow-auto rounded-md bg-tr-page/70 p-2 font-mono text-[10px] leading-relaxed"
      >{fullDetail}</pre>
    {/if}
  {/if}
</div>

<style>
  .status-banner {
    max-width: min(28rem, 48vw);
    flex-shrink: 1;
  }

  .status-banner-message {
    overflow-wrap: anywhere;
    word-break: break-word;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    overflow: hidden;
  }

  .status-banner-detail {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    word-break: break-word;
  }
</style>
