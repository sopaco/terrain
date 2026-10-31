<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { FolderOpen, Loader2, Network, RefreshCw, TriangleAlert } from "@lucide/svelte";
  import {
    getGraphifyStatus,
    getGraphifyView,
    openLocalPath,
    runGraphify,
  } from "../api";
  import { tr } from "../i18n";
  import { project, setProjectTask } from "../stores/project.svelte";
  import type { GraphifyStatus } from "../types";

  interface Props {
    repoPath: string | null;
    projectSlug: string | null;
    onStatus?: (
      message: string,
      kind: "idle" | "loading" | "progress" | "success" | "error",
    ) => void;
  }

  let { repoPath, projectSlug, onStatus }: Props = $props();

  let status = $state<GraphifyStatus | null>(null);
  let viewHtml = $state<string | null>(null);
  let viewTooLarge = $state(false);
  let viewLoading = $state(false);
  let statusLoading = $state(false);

  const task = $derived(
    projectSlug ? (project.projectTasks[projectSlug] ?? null) : null,
  );
  const busy = $derived(task?.graphifyBusy ?? false);
  const progressLine = $derived(task?.graphifyProgress ?? "");
  const hasGraph = $derived(status?.graph_present ?? false);

  async function loadStatus() {
    if (!repoPath) {
      status = null;
      viewHtml = null;
      return;
    }
    statusLoading = true;
    try {
      status = await getGraphifyStatus(repoPath);
      if (status.graph_present) {
        void loadView();
      } else {
        viewHtml = null;
      }
    } catch (e) {
      status = null;
      viewHtml = null;
      onStatus?.(String(e), "error");
    } finally {
      statusLoading = false;
    }
  }

  async function loadView() {
    if (!repoPath) return;
    viewLoading = true;
    try {
      const view = await getGraphifyView(repoPath);
      viewHtml = view.html;
      viewTooLarge = view.html_too_large;
    } catch {
      viewHtml = null;
      onStatus?.(tr("graph.loadFailed"), "error");
    } finally {
      viewLoading = false;
    }
  }

  async function run(action: "extract" | "update") {
    if (!repoPath || !projectSlug || busy) return;
    setProjectTask(projectSlug, { graphifyBusy: true, graphifyProgress: "" });
    viewHtml = null;
    try {
      await runGraphify(repoPath, projectSlug, action);
      // Completion (success or failure) arrives via the graphify-done event.
    } catch (e) {
      setProjectTask(projectSlug, { graphifyBusy: false, graphifyProgress: "" });
      onStatus?.(String(e), "error");
    }
  }

  function builtAt(secs: number | null): string {
    if (!secs) return tr("graph.statsAbsent");
    return new Date(secs * 1000).toLocaleString();
  }

  function stalenessChipClass(s: GraphifyStatus): string {
    if (s.staleness === "fresh") return "bg-tr-good-soft text-tr-on-good";
    if (s.staleness === "stale") return "bg-tr-watch-soft text-tr-on-watch";
    return "bg-tr-elevated text-tr-ink-3";
  }

  function stalenessLabel(s: GraphifyStatus): string {
    if (s.staleness === "fresh") return tr("graph.staleness.fresh");
    if (s.staleness === "stale") return tr("graph.staleness.stale");
    return tr("graph.staleness.unknown");
  }

  onMount(() => {
    const unsubs: (() => void)[] = [];
    void (async () => {
      unsubs.push(
        await listen<{
          project_slug: string;
          stage: string;
          message: string;
        }>("graphify-progress", (ev) => {
          if (projectSlug && ev.payload.project_slug === projectSlug) {
            setProjectTask(projectSlug, {
              graphifyProgress: ev.payload.message,
            });
          }
        }),
      );
      unsubs.push(
        await listen<{
          project_slug: string;
          ok: boolean;
          message: string;
        }>("graphify-done", (ev) => {
          if (projectSlug && ev.payload.project_slug === projectSlug) {
            setProjectTask(projectSlug, {
              graphifyBusy: false,
              graphifyProgress: "",
            });
            onStatus?.(
              ev.payload.message,
              ev.payload.ok ? "success" : "error",
            );
            void loadStatus();
          }
        }),
      );
    })();
    return () => unsubs.forEach((u) => u());
  });

  $effect(() => {
    void loadStatus();
  });
</script>

<div class="flex min-h-0 flex-1 flex-col gap-4 p-4">
    <header class="shrink-0">
      <h2 class="text-lg font-semibold text-tr-ink">{tr("graph.title")}</h2>
      <p class="mt-1 text-sm text-tr-ink-3">
        {tr("graph.descLine1")}
        <code class="text-tr-ink-2">{tr("graph.graphDir")}</code>
      </p>
    </header>

    {#if !repoPath}
      <p class="text-sm text-tr-ink-3">{tr("graph.noProject")}</p>
    {:else if statusLoading || !status}
      <div class="flex items-center gap-2 text-sm text-tr-ink-3" role="status" aria-busy="true">
        <Loader2 size={16} class="animate-spin" aria-hidden="true" />
        <span>{tr("graph.busy")}</span>
      </div>
    {:else}
      {#if !status.binary_available}
        <div
          class="shrink-0 rounded-xl border border-tr-border bg-tr-elevated px-4 py-4"
          role="note"
        >
          <div class="flex items-start gap-3">
            <TriangleAlert size={18} class="mt-0.5 shrink-0 text-tr-watch" aria-hidden="true" />
            <div class="min-w-0">
              <p class="text-sm font-medium text-tr-ink-2">{tr("graph.installTitle")}</p>
              <p class="mt-1 text-xs text-tr-ink-3">{tr("graph.installHint")}</p>
              <pre
                class="mt-2 overflow-x-auto rounded-lg bg-tr-raised px-3 py-2 text-xs text-tr-ink-2"
              >uv tool install graphifyy</pre>
              <p class="mt-1 text-[11px] text-tr-ink-3">pipx install graphifyy</p>
            </div>
          </div>
        </div>
      {/if}

      {#if hasGraph}
        {#if viewTooLarge}
          <div class="shrink-0 rounded-xl border border-tr-border bg-tr-elevated px-4 py-4">
            <div class="flex items-start gap-3">
              <TriangleAlert size={18} class="mt-0.5 shrink-0 text-tr-watch" aria-hidden="true" />
              <div class="min-w-0">
                <p class="text-sm font-medium text-tr-ink-2">{tr("graph.tooLargeTitle")}</p>
                <p class="mt-1 text-xs text-tr-ink-3">{tr("graph.tooLargeHint")}</p>
                <button
                  type="button"
                  class="tr-press mt-2 inline-flex items-center gap-1.5 rounded-lg border border-tr-border-strong px-3 py-1.5 text-xs transition-colors hover:bg-tr-raised"
                  onclick={() => openLocalPath(status?.out_dir ?? "")}
                >
                  <FolderOpen size={14} aria-hidden="true" />
                  {tr("graph.actions.openFolder")}
                </button>
              </div>
            </div>
          </div>
        {:else}
          <iframe
            title={tr("graph.title")}
            class="min-h-40 w-full flex-1 rounded-xl border border-tr-border bg-tr-page"
            srcdoc={viewHtml ?? ""}
            sandbox="allow-scripts"
          ></iframe>
        {/if}

        <div class="flex shrink-0 flex-wrap items-center gap-2 text-xs text-tr-ink-3">
          <span
            class={`rounded-full px-2.5 py-1 ${stalenessChipClass(status)}`}
          >
            {stalenessLabel(status)}
          </span>
          {#if status.nodes !== null}
            <span class="rounded-full bg-tr-elevated px-2.5 py-1">
              {tr("graph.stats.nodes")}: {status.nodes}
            </span>
            <span class="rounded-full bg-tr-elevated px-2.5 py-1">
              {tr("graph.stats.links")}: {status.links}
            </span>
          {/if}
          <span class="rounded-full bg-tr-elevated px-2.5 py-1">
            {tr("graph.stats.built")}: {builtAt(status.graph_mtime_secs)}
          </span>
          <button
            type="button"
            class="tr-press inline-flex items-center gap-1.5 rounded-lg bg-tr-accent px-3.5 py-1.5 font-medium text-tr-on-accent transition-colors hover:bg-tr-accent-hover disabled:opacity-50"
            disabled={busy}
            onclick={() => void run("update")}
          >
            {#if busy}
              <Loader2 size={14} class="animate-spin" aria-hidden="true" />
              {tr("graph.busy")}
            {:else}
              <RefreshCw size={14} aria-hidden="true" />
              {tr("graph.actions.update")}
            {/if}
          </button>
        </div>

        {#if busy && progressLine}
          <div
            class="shrink-0 overflow-hidden rounded-lg border border-tr-accent-soft bg-tr-accent-soft px-3 py-2"
            role="status"
            aria-live="polite"
          >
            <p class="truncate text-xs text-tr-ink-2">{progressLine || tr("graph.busy")}</p>
          </div>
        {/if}

        {#if status.communities && status.communities.length > 0}
          <div class="flex shrink-0 flex-wrap items-center gap-2">
            <span class="text-xs font-medium text-tr-ink-3">{tr("graph.communities")}</span>
            {#each status.communities as community (community.name)}
              <span class="rounded-full bg-tr-elevated px-2.5 py-1 text-[11px] text-tr-ink-2">
                {community.name} · {community.size}
              </span>
            {/each}
          </div>
        {/if}
      {:else if !busy}
        <div
          class="flex min-h-48 flex-1 flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-tr-border-strong bg-tr-surface/50 px-6 py-10 text-center"
        >
          <Network size={28} class="text-tr-ink-4" aria-hidden="true" />
          <p class="text-sm font-medium text-tr-ink-2">{tr("graph.missingGraphTitle")}</p>
          <p class="max-w-md text-xs text-tr-ink-3">{tr("graph.missingGraphHint")}</p>
          <button
            type="button"
            class="tr-press inline-flex items-center gap-1.5 rounded-lg bg-tr-accent px-4 py-2 text-sm font-medium text-tr-on-accent transition-colors hover:bg-tr-accent-hover disabled:opacity-50"
            onclick={() => void run("extract")}
          >
            <Network size={15} aria-hidden="true" />
            {tr("graph.actions.generate")}
          </button>
        </div>
      {:else}
        <div
          class="flex flex-1 items-center justify-center gap-2 text-sm text-tr-ink-3"
          role="status"
          aria-live="polite"
          aria-busy="true"
        >
          <Loader2 size={16} class="animate-spin" aria-hidden="true" />
          <span>{tr("graph.busy")}</span>
        </div>
      {/if}
    {/if}
</div>