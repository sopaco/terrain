<script lang="ts">
    import { CircleAlert, Settings2, TriangleAlert } from "@lucide/svelte";

    import ChevronIcon from "./icons/ChevronIcon.svelte";

    export interface OverviewActionItem {
        id: string;
        priority: number;
        accent: "rose" | "amber" | "violet";
        title: string;
        detail: string;
        hint?: string;
        actionLabel: string;
        busyLabel?: string;
        onAction?: () => void;
        disabled?: boolean;
        busy?: boolean;
    }

    interface Props {
        items: OverviewActionItem[];
        progressNote?: string | null;
        /** When false, show every item expanded (e.g. global stale repair list). */
        collapseSecondary?: boolean;
    }

    let { items, progressNote = null, collapseSecondary = true }: Props = $props();

    let moreOpen = $state(false);

    const sorted = $derived([...items].sort((a, b) => a.priority - b.priority));
    const primary = $derived(sorted[0] ?? null);
    const secondary = $derived(sorted.slice(1));

    const accentTint: Record<OverviewActionItem["accent"], string> = {
        rose: "bg-tr-critical-soft border-tr-critical/30",
        amber: "bg-tr-watch-soft border-tr-watch/30",
        violet: "bg-tr-accent-soft border-tr-accent-soft-strong",
    };

    const accentIconTint: Record<OverviewActionItem["accent"], string> = {
        rose: "bg-tr-critical/20 text-tr-critical",
        amber: "bg-tr-watch/20 text-tr-watch",
        violet: "bg-tr-accent-soft text-tr-accent",
    };

    const accentIcon = {
        rose: TriangleAlert,
        amber: CircleAlert,
        violet: Settings2,
    } as const;

    const accentButton: Record<OverviewActionItem["accent"], string> = {
        rose: "bg-tr-critical text-tr-on-critical hover:opacity-90",
        amber: "bg-tr-watch text-tr-on-watch hover:opacity-90",
        violet: "bg-tr-accent text-tr-on-accent hover:bg-tr-accent-hover",
    };
</script>

{#snippet actionRow(item: OverviewActionItem, compact = false)}
    {@const AccentIcon = accentIcon[item.accent]}
    <div
        class={`flex items-center gap-3 rounded-xl border px-4 ${compact ? "py-3" : "py-3.5"} ${accentTint[item.accent]}`}
    >
        <span
            class={`flex h-9 w-9 shrink-0 items-center justify-center rounded-lg ${accentIconTint[item.accent]}`}
            aria-hidden="true"
        >
            <AccentIcon size={20} strokeWidth={2} />
        </span>
        <div class="min-w-0 flex-1">
            <p class="text-sm font-medium text-tr-ink">
                {item.title}
            </p>
            <p class="mt-0.5 text-xs leading-relaxed text-tr-ink-2">
                {item.detail}
            </p>
            {#if item.hint}
                <p class="mt-1.5 text-[11px] text-tr-ink-3">
                    {item.hint}
                </p>
            {/if}
            {#if progressNote}
                <p class="mt-1.5 text-xs text-tr-accent">
                    {progressNote}
                </p>
            {/if}
        </div>
        {#if item.onAction}
            <button
                type="button"
                class={`shrink-0 rounded-lg px-3.5 py-2 text-xs font-medium disabled:opacity-50 ${accentButton[item.accent]}`}
                disabled={item.disabled || item.busy}
                onclick={item.onAction}
            >
                {item.busy
                    ? (item.busyLabel ?? "处理中…")
                    : item.actionLabel}
            </button>
        {/if}
    </div>
{/snippet}

{#if primary}
    <div class="space-y-2">
        {@render actionRow(primary)}

        {#if secondary.length > 0}
            {#if collapseSecondary}
                <button
                    type="button"
                    class="flex w-full items-center gap-2 rounded-lg px-1 py-1.5 text-left text-xs text-tr-ink-3 transition-colors hover:text-tr-ink-2"
                    aria-expanded={moreOpen}
                    onclick={() => (moreOpen = !moreOpen)}
                >
                    <ChevronIcon
                        direction={moreOpen ? "down" : "right"}
                        size={12}
                        class="shrink-0 text-tr-ink-3"
                    />
                    还有 {secondary.length} 项待处理
                </button>

                {#if moreOpen}
                    <div class="space-y-2">
                        {#each secondary as item (item.id)}
                            {@render actionRow(item, true)}
                        {/each}
                    </div>
                {/if}
            {:else}
                <div class="space-y-2">
                    {#each secondary as item (item.id)}
                        {@render actionRow(item, true)}
                    {/each}
                </div>
            {/if}
        {/if}
    </div>
{/if}
