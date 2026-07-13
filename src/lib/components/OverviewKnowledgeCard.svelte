<script lang="ts">
  import { BookOpen, Compass } from "@lucide/svelte";

  type IconKind = "compass" | "book";

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
    nested?: boolean;
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
    nested = false,
  }: Props = $props();
</script>

<div
  class={`flex flex-col p-4 ${
    nested
      ? "rounded-xl border border-white/8 bg-white/[0.02]"
      : "rounded-2xl border border-white/8 bg-[#14171c]"
  }`}
>
  <div class="flex items-start gap-3">
    <span
      class={`flex h-10 w-10 shrink-0 items-center justify-center rounded-xl ${
        ready ? "bg-indigo-500/15 text-indigo-300" : "bg-white/5 text-white/35"
      }`}
      aria-hidden="true"
    >
      {#if icon === "compass"}
        <Compass size={20} strokeWidth={1.75} />
      {:else}
        <BookOpen size={20} strokeWidth={1.75} />
      {/if}
    </span>
    <div class="min-w-0 flex-1">
      <div class="flex items-start justify-between gap-2">
        <p class="text-sm font-medium text-white/90">{title}</p>
        <span
          class={`shrink-0 rounded-full px-2 py-0.5 text-[10px] font-medium ${
            ready ? "bg-emerald-500/15 text-emerald-200" : "bg-white/8 text-white/45"
          }`}
        >
          {ready ? "已就绪" : "待处理"}
        </span>
      </div>
      <p class="mt-0.5 text-xs leading-relaxed text-white/45">{subtitle}</p>
      <p class="mt-2 text-xs text-white/55">{meta}</p>
    </div>
  </div>

  {#if onPrimary}
    <div class="mt-3 flex items-center justify-end gap-2 border-t border-white/8 pt-3">
      {#if secondaryLabel && onSecondary}
        <button
          type="button"
          class="rounded-lg px-2.5 py-1.5 text-xs text-indigo-300/90 hover:text-indigo-200 disabled:opacity-50"
          disabled={secondaryDisabled}
          onclick={onSecondary}
        >
          {secondaryLabel}
        </button>
      {/if}
      <button
        type="button"
        class="rounded-lg bg-indigo-600 px-3 py-1.5 text-xs font-medium hover:bg-indigo-500 disabled:opacity-50"
        disabled={primaryDisabled || !onPrimary}
        onclick={onPrimary}
      >
        {primaryLabel}
      </button>
    </div>
  {/if}
</div>
