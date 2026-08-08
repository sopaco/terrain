<script lang="ts">
  import { Check, Download, Image } from "@lucide/svelte";

  interface Props {
    /** `copy` puts one page on the clipboard; `export` writes every page to disk. */
    mode?: "copy" | "export";
    copied?: boolean;
    copying?: boolean;
    onclick: () => void;
  }

  let { mode = "copy", copied = false, copying = false, onclick }: Props = $props();

  const isExport = $derived(mode === "export");

  const label = $derived(
    copied
      ? isExport
        ? "已导出"
        : "已复制"
      : copying
        ? isExport
          ? "导出中…"
          : "生成中…"
        : isExport
          ? "导出长图"
          : "复制图片",
  );

  const title = $derived(
    isExport ? "导出为 PNG 文件（长回答自动分页）" : "复制为图片（长回答复制第 1 页）",
  );

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
  {title}
  onclick={(event) => {
    event.preventDefault();
    onclick();
  }}
>
  {#if copied}
    <Check size={12} strokeWidth={2.5} class="shrink-0" aria-hidden="true" />
  {:else if copying}
    <span
      class="inline-block h-3 w-3 shrink-0 animate-spin rounded-full border border-tr-border-strong border-t-white/80"
      aria-hidden="true"
    ></span>
  {:else if isExport}
    <Download size={12} strokeWidth={2.25} class="shrink-0" aria-hidden="true" />
  {:else}
    <Image size={12} strokeWidth={2.25} class="shrink-0" aria-hidden="true" />
  {/if}
  {label}
</button>
