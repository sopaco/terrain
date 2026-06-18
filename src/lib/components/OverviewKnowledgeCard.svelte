<script lang="ts">
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

{#snippet iconSvg(kind: IconKind)}
  {#if kind === "compass"}
    <svg class="h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" aria-hidden="true">
      <circle cx="12" cy="12" r="9" />
      <path d="M14.5 9.5 10 14l4.5-4.5Z" fill="currentColor" stroke="none" />
      <path d="m10 14 1.5 4.5L14.5 14" />
    </svg>
  {:else}
    <svg class="h-5 w-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" aria-hidden="true">
      <path d="M5 5.5A2.5 2.5 0 0 1 7.5 3H18v18H7.5A2.5 2.5 0 0 1 5 18.5V5.5Z" />
      <path d="M5 5.5A2.5 2.5 0 0 0 7.5 3H18" />
      <path d="M9 7h6M9 11h6" />
    </svg>
  {/if}
{/snippet}

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
      {@render iconSvg(icon)}
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
