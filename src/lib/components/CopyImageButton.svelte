<script lang="ts">
  import { Check, Image } from "@lucide/svelte";

  interface Props {
    copied?: boolean;
    copying?: boolean;
    onclick: () => void;
  }

  let { copied = false, copying = false, onclick }: Props = $props();

  const buttonClass = $derived(
    copied
      ? "border-tr-good/35 bg-tr-good-soft text-tr-good scale-105"
      : copying
        ? "cursor-wait border-tr-border-strong bg-tr-elevated text-tr-ink-3 opacity-70"
        : "border-tr-border-strong bg-tr-elevated text-tr-ink-3 hover:border-tr-accent-soft-strong hover:bg-tr-accent-soft hover:text-tr-accent-hover",
  );
</script>

<button
  type="button"
  class="inline-flex shrink-0 items-center gap-1 rounded-md border px-2 py-1 text-[10px] transition-[color,background-color,border-color,transform] duration-150 {buttonClass}"
  disabled={copying}
  aria-live="polite"
  title="复制为图片"
  onclick={(event) => {
    event.preventDefault();
    onclick();
  }}
>
  {#if copied}
    <Check size={12} strokeWidth={2.5} class="shrink-0" aria-hidden="true" />
    已复制
  {:else if copying}
    <span
      class="inline-block h-3 w-3 shrink-0 animate-spin rounded-full border border-tr-border-strong border-t-white/80"
      aria-hidden="true"
    ></span>
    生成中…
  {:else}
    <Image size={12} strokeWidth={2.25} class="shrink-0" aria-hidden="true" />
    复制图片
  {/if}
</button>
