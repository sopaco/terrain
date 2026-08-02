<script lang="ts">
  import { BookOpen, Compass, ListTree } from "@lucide/svelte";

  type IconKind = "compass" | "book" | "list";

  interface Props {
    title: string;
    subtitle: string;
    meta: string;
    ready: boolean;
    icon: IconKind;
    primaryLabel: string;
    onPrimary?: () => void;
    primaryDisabled?: boolean;
    secondaryLabel?: string;
    onSecondary?: () => void;
    secondaryDisabled?: boolean;
  }

  let {
    title,
    subtitle,
    meta,
    ready,
    icon,
    primaryLabel,
    onPrimary,
    primaryDisabled = false,
    secondaryLabel,
    onSecondary,
    secondaryDisabled = false,
  }: Props = $props();
</script>

<div
  class="flex flex-col gap-3 rounded-xl border border-tr-border bg-tr-surface p-4"
>
  <div class="flex items-start gap-3">
    <span
      class={`flex h-9 w-9 shrink-0 items-center justify-center rounded-lg ${
        ready ? "bg-tr-accent-soft text-tr-accent" : "bg-tr-elevated text-tr-ink-3"
      }`}
      aria-hidden="true"
    >
      {#if icon === "compass"}
        <Compass size={18} strokeWidth={1.75} />
      {:else if icon === "book"}
        <BookOpen size={18} strokeWidth={1.75} />
      {:else}
        <ListTree size={18} strokeWidth={1.75} />
      {/if}
    </span>
    <div class="min-w-0 flex-1">
      <div class="flex items-start justify-between gap-2">
        <p class="text-sm font-medium text-tr-ink">{title}</p>
        <span
          class={`shrink-0 rounded-full px-2 py-0.5 text-[10px] font-medium ${
            ready ? "bg-tr-good-soft text-tr-good" : "bg-tr-elevated text-tr-ink-3"
          }`}
        >
          {ready ? "已就绪" : "待处理"}
        </span>
      </div>
      <p class="mt-0.5 text-[11.5px] leading-relaxed text-tr-ink-3">{subtitle}</p>
      <p class="mt-2 text-xs text-tr-ink-2">{meta}</p>
    </div>
  </div>

  {#if onPrimary}
    <div class="flex items-center justify-end gap-2 border-t border-tr-border pt-3">
      {#if secondaryLabel && onSecondary}
        <button
          type="button"
          class="tr-press rounded-lg px-2 py-1.5 text-xs text-tr-ink-3 transition-colors hover:text-tr-ink-2 disabled:opacity-50"
          disabled={secondaryDisabled}
          onclick={onSecondary}
        >
          {secondaryLabel}
        </button>
      {/if}
      <button
        type="button"
        class="tr-press rounded-lg bg-tr-accent px-3 py-1.5 text-xs font-medium text-tr-on-accent transition-colors hover:bg-tr-accent-hover disabled:opacity-50"
        disabled={primaryDisabled || !onPrimary}
        onclick={onPrimary}
      >
        {primaryLabel}
      </button>
    </div>
  {/if}
</div>
