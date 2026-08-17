<script lang="ts">
  export type StatusKind = "idle" | "loading" | "progress" | "success" | "error";

  import { Check, CircleAlert } from "@lucide/svelte";
  import { tr } from "../i18n";

  interface Props {
    message: string;
    kind?: StatusKind;
    detail?: string | null;
  }

  let { message, kind = "idle", detail = null }: Props = $props();

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
</script>

<div
  role="status"
  class={`status-banner flex items-start gap-2 rounded-lg border px-2.5 py-1.5 text-xs ${chipStyles[kind]}`}
  title={detail ?? message}
>
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
  <p class="status-banner-message m-0 leading-snug">{message}</p>
</div>

<style>
  .status-banner {
    width: max-content;
    flex-shrink: 0;
    overflow: visible;
  }

  .status-banner-message {
    overflow: visible;
    text-overflow: clip;
    white-space: normal;
    overflow-wrap: anywhere;
    word-break: break-word;
  }
</style>
