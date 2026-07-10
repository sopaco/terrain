<script lang="ts">
  import { onMount } from "svelte";
  import { getUsageSnapshot } from "../api";
  import type { UsageSnapshot } from "../types";
  import UsageBadge from "./UsageBadge.svelte";
  import UsagePanel from "./UsagePanel.svelte";

  const DEFER_MS = 4_000;

  let snapshot = $state<UsageSnapshot | null>(null);
  let loading = $state(false);
  let panelOpen = $state(false);
  let started = $state(false);

  const initialSnapshot = $derived(snapshot);

  onMount(() => {
    const timer = window.setTimeout(() => {
      started = true;
      void loadSummary(false);
    }, DEFER_MS);
    return () => window.clearTimeout(timer);
  });

  async function loadSummary(force: boolean) {
    if (loading) return;
    loading = true;
    try {
      snapshot = await getUsageSnapshot("summary", force);
    } catch {
      // Badge stays quiet on failure — drawer can retry.
    } finally {
      loading = false;
    }
  }

  function openPanel() {
    panelOpen = true;
  }

  function closePanel() {
    panelOpen = false;
    if (started) void loadSummary(false);
  }
</script>

{#if started}
  <UsageBadge {snapshot} {loading} onclick={openPanel} />
{/if}

<UsagePanel open={panelOpen} {initialSnapshot} onclose={closePanel} />
