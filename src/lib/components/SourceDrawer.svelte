<script lang="ts">
  import { onMount } from "svelte";
  import SourcePanel from "./SourcePanel.svelte";
  import type { SourceCitation, SourceSlice } from "../types";

  interface Props {
    open: boolean;
    slice: SourceSlice | null;
    repoPath?: string | null;
    onclose: () => void;
    onSourceClick?: (citation: SourceCitation) => void;
  }

  let { open, slice, repoPath = null, onclose, onSourceClick }: Props = $props();

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && open) {
      onclose();
    }
  }

  onMount(() => {
    window.addEventListener("keydown", onKeydown);
    return () => window.removeEventListener("keydown", onKeydown);
  });
</script>

{#if open && slice}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-40 bg-black/50 backdrop-blur-[2px]"
    onclick={onclose}
    role="presentation"
  ></div>

  <section
    class="fixed inset-y-0 right-0 z-50 flex w-[70vw] flex-col border-l border-white/10 bg-[#10131a] shadow-2xl"
    aria-label="Source"
  >
    <SourcePanel {slice} {repoPath} {onclose} {onSourceClick} />
  </section>
{/if}
