<script lang="ts">
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
    <svg class="h-3 w-3 shrink-0" viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path
        d="M3.5 8.5L6.5 11.5L12.5 4.5"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
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
