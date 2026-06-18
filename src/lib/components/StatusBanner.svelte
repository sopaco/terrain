<script lang="ts">
  export type StatusKind = "idle" | "loading" | "progress" | "success" | "error";

  import { STATUS_CHIP_LABELS } from "../terminology";

  interface Props {
    message: string;
    kind?: StatusKind;
    detail?: string | null;
  }

  let { message, kind = "idle", detail = null }: Props = $props();

  const chipStyles: Record<StatusKind, string> = {
    idle: "border-white/10 bg-white/5 text-white/55",
    loading: "border-sky-500/30 bg-sky-500/10 text-sky-200",
    progress: "border-amber-500/30 bg-amber-500/10 text-amber-100",
    success: "border-emerald-500/30 bg-emerald-500/10 text-emerald-200",
    error: "border-rose-500/30 bg-rose-500/10 text-rose-200",
  };

  const labels = STATUS_CHIP_LABELS;
</script>

<div
  role="status"
  class={`status-banner flex items-start gap-2 rounded-lg border px-2.5 py-1.5 text-xs ${chipStyles[kind]}`}
  title={detail ?? message}
>
  <div class="flex shrink-0 items-center gap-1.5 pt-px">
    {#if kind === "loading" || kind === "progress"}
      <span
        class="inline-block h-3 w-3 shrink-0 animate-spin rounded-full border border-current border-t-transparent"
      ></span>
    {:else if kind === "success"}
      <span class="text-[10px]">✓</span>
    {:else if kind === "error"}
      <span class="text-[10px]">!</span>
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
