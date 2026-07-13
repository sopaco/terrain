<script lang="ts">
  import { ChevronDown, ChevronRight, ChevronUp } from "@lucide/svelte";

  type ChevronDirection = "right" | "down" | "up";

  interface Props {
    direction?: ChevronDirection;
    size?: number;
    class?: string;
  }

  let {
    direction = "right",
    size = 14,
    class: className = "shrink-0 text-white/40",
  }: Props = $props();
</script>

{#if direction === "down"}
  <ChevronDown {size} strokeWidth={2} class={className} aria-hidden="true" />
{:else if direction === "up"}
  <ChevronUp {size} strokeWidth={2} class={className} aria-hidden="true" />
{:else}
  <ChevronRight {size} strokeWidth={2} class={className} aria-hidden="true" />
{/if}
