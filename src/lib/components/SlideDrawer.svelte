<script lang="ts">
  import type { Snippet } from "svelte";
  import {
    handleDismissTransitionEnd,
    prefersReducedMotion,
    schedulePresent,
  } from "../overlayMotion";

  interface Props {
    open: boolean;
    onclose: () => void;
    ariaLabel: string;
    widthClass?: string;
    panelClass?: string;
    zBackdrop?: number;
    zPanel?: number;
    side?: "left" | "right";
    children: Snippet;
  }

  let {
    open,
    onclose,
    ariaLabel,
    widthClass = "w-[70vw]",
    panelClass = "",
    zBackdrop = 40,
    zPanel = 50,
    side = "right",
    children,
  }: Props = $props();

  let mounted = $state(false);
  let presented = $state(false);
  let showContent = $state(false);

  const panelPositionClass = $derived(
    side === "right" ? "right-0 border-l" : "left-0 border-r",
  );

  $effect(() => {
    if (open) {
      mounted = true;
      showContent = false;
      void schedulePresent((value) => {
        presented = value;
        if (value) showContent = true;
      });
      return;
    }
    presented = false;
    if (prefersReducedMotion()) {
      showContent = false;
      mounted = false;
    }
  });

  function onDrawerTransitionEnd(e: TransitionEvent) {
    handleDismissTransitionEnd(e, presented, () => {
      showContent = false;
      mounted = false;
    });
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && open) onclose();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if mounted}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="mm-overlay-backdrop fixed inset-0 bg-black/55"
    class:is-presented={presented}
    style:z-index={zBackdrop}
    onclick={onclose}
    role="presentation"
  ></div>

  <section
    class="mm-overlay-drawer fixed inset-y-0 flex flex-col border-white/10 bg-[#10131a] shadow-2xl {panelPositionClass} {widthClass} {panelClass}"
    class:is-presented={presented}
    style:z-index={zPanel}
    aria-label={ariaLabel}
    ontransitionend={onDrawerTransitionEnd}
  >
    {#if showContent}
      {@render children()}
    {/if}
  </section>
{/if}
