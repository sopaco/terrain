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
    ariaLabelledby?: string;
    dialogClass?: string;
    zBackdrop?: number;
    zDialog?: number;
    children: Snippet;
  }

  let {
    open,
    onclose,
    ariaLabelledby,
    dialogClass = "",
    zBackdrop = 300,
    zDialog = 301,
    children,
  }: Props = $props();

  let mounted = $state(false);
  let presented = $state(false);
  let showContent = $state(false);

  $effect(() => {
    if (open) {
      mounted = true;
      showContent = true;
      void schedulePresent((value) => {
        presented = value;
      });
      return;
    }
    presented = false;
    if (prefersReducedMotion()) {
      showContent = false;
      mounted = false;
    }
  });

  function onModalTransitionEnd(e: TransitionEvent) {
    handleDismissTransitionEnd(e, presented, () => {
      showContent = false;
      mounted = false;
    }, ["transform", "opacity"]);
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

  <div
    class="fixed inset-0 flex items-center justify-center p-4 pointer-events-none"
    style:z-index={zDialog}
  >
    <div
      class="mm-overlay-modal pointer-events-auto flex max-h-[min(90vh,720px)] w-full max-w-[min(92vw,560px)] flex-col overflow-hidden rounded-2xl border border-tr-border-strong bg-tr-raised shadow-2xl {dialogClass}"
      class:is-presented={presented}
      role="dialog"
      aria-modal="true"
      aria-labelledby={ariaLabelledby}
      ontransitionend={onModalTransitionEnd}
    >
      {#if showContent}
        {@render children()}
      {/if}
    </div>
  </div>
{/if}
