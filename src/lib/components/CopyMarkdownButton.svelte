<script lang="ts">
  import { Check } from "@lucide/svelte";

  interface Props {
    copied?: boolean;
    copying?: boolean;
    onclick: () => void;
  }

  let { copied = false, copying = false, onclick }: Props = $props();

  const buttonClass = $derived(
    copied
      ? "border-emerald-500/40 bg-emerald-500/15 text-emerald-300 scale-105"
      : copying
        ? "cursor-wait border-white/10 bg-white/[0.03] text-white/45 opacity-70"
        : "border-white/10 bg-white/[0.03] text-white/45 hover:border-indigo-500/40 hover:bg-indigo-500/10 hover:text-indigo-200",
  );
</script>

<button
  type="button"
  class="inline-flex shrink-0 items-center gap-1 rounded-md border px-2 py-1 text-[10px] transition-all duration-200 {buttonClass}"
  disabled={copying}
  aria-live="polite"
  {onclick}
>
  {#if copied}
    <Check size={12} strokeWidth={2.5} class="shrink-0" aria-hidden="true" />
    已复制
  {:else if copying}
    <span
      class="inline-block h-3 w-3 shrink-0 animate-spin rounded-full border border-white/30 border-t-white/80"
      aria-hidden="true"
    ></span>
    复制中…
  {:else}
    复制 Markdown
  {/if}
</button>
