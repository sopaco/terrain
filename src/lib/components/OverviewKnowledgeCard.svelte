<script lang="ts">
  import { BookOpen, Compass, ListTree, RefreshCw } from "@lucide/svelte";
  import { tr } from "../i18n";

  type IconKind = "compass" | "book" | "list";

  interface Props {
    title: string;
    subtitle: string;
    meta: string;
    ready: boolean;
    icon: IconKind;
    /** Visually elevate this card as the recommended entry point. */
    featured?: boolean;
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
    featured = false,
    primaryLabel,
    onPrimary,
    primaryDisabled = false,
    secondaryLabel,
    onSecondary,
    secondaryDisabled = false,
  }: Props = $props();

  // Each asset kind gets its own semantic icon color so the three cards read
  // as distinct at a glance instead of three identical teal chips.
  const iconClass = $derived(
    (() => {
      if (!ready) return "bg-tr-elevated text-tr-ink-3";
      if (icon === "compass") return "bg-tr-accent-soft text-tr-accent";
      if (icon === "book") return "bg-tr-good-soft text-tr-good";
      return "bg-tr-watch-soft text-tr-watch";
    })(),
  );
</script>

<div
  class={`flex flex-col gap-3 rounded-xl border p-4 transition-colors ${
    featured
      ? "border-tr-accent-soft-strong bg-tr-accent-soft"
      : "border-tr-border bg-tr-surface"
  }`}
>
  <div class="flex items-start gap-3">
    <span
      class={`flex h-9 w-9 shrink-0 items-center justify-center rounded-lg ${iconClass}`}
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
        <div class="flex items-center gap-1.5">
          <p class="text-sm font-medium text-tr-ink">{title}</p>
          {#if featured}
            <span
              class="rounded-full bg-tr-accent px-1.5 py-0.5 text-[9px] font-semibold text-tr-on-accent"
              >{tr("overview.card.featured")}</span
            >
          {/if}
        </div>
        <span
          class={`shrink-0 rounded-full px-2 py-0.5 text-[10px] font-medium ${
            ready ? "bg-tr-good-soft text-tr-good" : "bg-tr-elevated text-tr-ink-3"
          }`}
        >
          {ready ? tr("overview.card.ready") : tr("overview.card.pending")}
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
          class="tr-press inline-flex items-center gap-1 rounded-lg border border-tr-border-strong px-2.5 py-1.5 text-xs text-tr-ink-3 transition-colors hover:bg-tr-elevated hover:text-tr-ink-2 disabled:opacity-50"
          disabled={secondaryDisabled}
          title={tr("overview.card.regenerateTitle")}
          onclick={onSecondary}
        >
          <RefreshCw size={12} strokeWidth={2} aria-hidden="true" />
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
