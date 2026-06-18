<script lang="ts">
  import SourcePanel from "./SourcePanel.svelte";
  import SlideDrawer from "./SlideDrawer.svelte";
  import type { SourceCitation, SourceSlice } from "../types";

  interface Props {
    open: boolean;
    slice: SourceSlice | null;
    repoPath?: string | null;
    onclose: () => void;
    onSourceClick?: (citation: SourceCitation) => void;
  }

  let { open, slice, repoPath = null, onclose, onSourceClick }: Props = $props();
</script>

<SlideDrawer {open} {onclose} ariaLabel="Source" widthClass="w-[70vw]">
  {#if slice}
    <SourcePanel {slice} {repoPath} {onclose} {onSourceClick} />
  {/if}
</SlideDrawer>
