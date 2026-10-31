<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { ask, open as openFolderDialog } from "@tauri-apps/plugin-dialog";
  import {
    EllipsisVertical,
    FolderOpen,
    Gauge,
    Loader2,
    Plus,
    RefreshCw,
    TriangleAlert,
  } from "@lucide/svelte";
  import {
    addProjects,
    getDashboardSummary,
    openLocalPath,
    removeProject,
    runDashboardBatch,
  } from "../api";
  import { tr } from "../i18n";
  import type {
    BatchUpdateResult,
    DashboardRow,
    DashboardSummary,
    ProjectRegistryStatus,
  } from "../types";

  interface Props {
    onStatus?: (
      message: string,
      kind: "idle" | "loading" | "progress" | "success" | "error",
    ) => void;
    onProjectsChanged?: () => void;
  }

  let { onStatus, onProjectsChanged }: Props = $props();

  let summary = $state<DashboardSummary | null>(null);
  let batchRunning = $state(false);
  let adding = $state(false);
  let batchLine = $state("");
  let busySlugs = $state<Set<string>>(new Set());
  let menuOpenSlug = $state<string | null>(null);

  const rows: DashboardRow[] = $derived(summary?.rows ?? []);
  const collaborations = $derived(summary?.collaborations ?? []);

  const metricCards = $derived.by(() => {
    const totals = summary?.totals;
    if (!totals) return [];
    return [
      { key: "projects", value: Number(totals.projects) },
      { key: "graphs", value: Number(totals.with_graph) },
      { key: "nodes", value: Number(totals.total_nodes) },
      { key: "links", value: Number(totals.total_links) },
      { key: "stale", value: Number(totals.stale) },
    ];
  });

  async function loadSummary() {
    try {
      summary = await getDashboardSummary();
    } catch (e) {
      summary = null;
      onStatus?.(String(e), "error");
    }
  }

  async function runBatch(slugs?: string[]) {
    if (batchRunning) return;
    batchRunning = true;
    batchLine = "";
    try {
      const result = await runDashboardBatch(slugs ?? null);
      onStatus?.(
        tr("dashboard.batchDone", {
          succeeded: Number(result.succeeded),
          failed: Number(result.failed),
        }),
        Number(result.failed) === 0 ? "success" : "idle",
      );
      await loadSummary();
    } catch (e) {
      onStatus?.(String(e), "error");
    } finally {
      batchRunning = false;
      busySlugs = new Set();
    }
  }

  async function addProjectsFlow() {
    if (adding || batchRunning) return;
    adding = true;
    try {
      const picked = await openFolderDialog({ directory: true, multiple: true });
      if (!picked || picked.length === 0) {
        onStatus?.(tr("dashboard.add.nonePicked"), "idle");
        return;
      }
      const result = await addProjects(picked);
      const added = result.added.filter((a) => a.scan_ok).length;
      if (added > 0) {
        onProjectsChanged?.();
        onStatus?.(tr("dashboard.add.resultAdded", { count: added }), "success");
      }
      if (result.added.length > added) {
        onStatus?.(
          tr("dashboard.add.resultScanWarn", { count: result.added.length - added }),
          "idle",
        );
      }
      if (result.errors.length > 0) {
        onStatus?.(
          tr("dashboard.add.resultErrors", {
            errors: result.errors.map((er) => `${er.path}: ${er.error}`).join("; "),
          }),
          "error",
        );
      }
      await loadSummary();
    } catch (e) {
      onStatus?.(String(e), "error");
    } finally {
      adding = false;
    }
  }

  function statusChipClass(status: ProjectRegistryStatus): string {
    if (status === "ready") return "bg-tr-good-soft text-tr-on-good";
    if (status === "partial") return "bg-tr-watch-soft text-tr-on-watch";
    return "bg-tr-critical-soft text-tr-on-critical";
  }

  function statusLabel(status: ProjectRegistryStatus): string {
    if (status === "ready") return tr("dashboard.status.ready");
    if (status === "partial") return tr("dashboard.status.partial");
    return tr("dashboard.status.stale");
  }

  function stalenessChipClass(row: DashboardRow): string {
    if (row.staleness === "fresh") return "bg-tr-good-soft text-tr-on-good";
    if (row.staleness === "stale") return "bg-tr-watch-soft text-tr-on-watch";
    return "bg-tr-elevated text-tr-ink-3";
  }

  function stalenessLabel(row: DashboardRow): string {
    if (row.staleness === "fresh") return tr("dashboard.graph.fresh");
    if (row.staleness === "stale") return tr("dashboard.graph.stale");
    return tr("dashboard.graph.unknown");
  }

  function graphCell(row: DashboardRow): string {
    if (!row.graph.graph_present) return tr("dashboard.graph.none");
    const nodes = row.graph.nodes;
    const links = row.graph.links;
    if (nodes === null || links === null) return tr("dashboard.graph.none");
    return `${Number(nodes)} / ${Number(links)}`;
  }

  function lastBatchCell(row: DashboardRow): string {
    const rec = row.last_batch;
    if (!rec) return tr("dashboard.batch.never");
    const when = new Date(Number(rec.finished_at_secs) * 1000).toLocaleString();
    return `${rec.ok ? "✓" : "✗"} ${tr("dashboard.batch.duration", { secs: rec.duration_secs })} · ${when}`;
  }

  const docsLegend = [
    { key: "agentContext", field: "agent_context" },
    { key: "humanDocs", field: "human_docs" },
    { key: "repomix", field: "repomix" },
    { key: "graph", field: "graph" },
    { key: "wiki", field: "wiki" },
    { key: "vault", field: "vault" },
  ] as const;

  /** Per-project max weekly commits, for scaling each small-multiple sparkline. */
  const weeklyMax = $derived.by(() => {
    const m = new Map<string, number>();
    for (const r of rows) {
      let max = 1;
      for (const w of r.activity.weekly_commits) max = Math.max(max, Number(w));
      m.set(r.slug, max);
    }
    return m;
  });

  const maxShared = $derived.by(() => {
    let max = 0;
    for (const c of collaborations) max = Math.max(max, Number(c.shared_authors));
    return max;
  });

  function docPresent(row: DashboardRow): number {
    let present = 0;
    for (const f of docsLegend) if (row.docs[f.field]) present += 1;
    return present;
  }

  function toggleMenu(slug: string) {
    menuOpenSlug = menuOpenSlug === slug ? null : slug;
  }

  function onWindowClick(ev: MouseEvent) {
    if (!menuOpenSlug) return;
    const target = ev.target as HTMLElement | null;
    if (target?.closest("[data-kebab]") || target?.closest("[data-menu-panel]")) return;
    menuOpenSlug = null;
  }

  async function updateRowGraph(row: DashboardRow) {
    menuOpenSlug = null;
    await runBatch([row.slug]);
  }

  function openRowFolder(row: DashboardRow) {
    menuOpenSlug = null;
    void openLocalPath(row.repo_path);
  }

  async function deleteProject(row: DashboardRow) {
    menuOpenSlug = null;
    try {
      const yes = await ask(tr("dashboard.delete.confirmBody", { name: row.name }), {
        title: tr("dashboard.delete.confirmTitle"),
        kind: "warning",
      });
      if (!yes) return;
      await removeProject(row.slug);
      onProjectsChanged?.();
      await loadSummary();
    } catch (e) {
      onStatus?.(String(e), "error");
    }
  }

  onMount(() => {
    const unsubs: (() => void)[] = [];
    void (async () => {
      unsubs.push(
        await listen<{ project_slug: string; project_name: string; stage: string; message: string }>(
          "dashboard-batch-progress",
          (ev) => {
            const { project_slug, stage, message } = ev.payload;
            if (stage === "started" || stage === "run" || stage === "skipped") {
              const next = new Set(busySlugs);
              next.add(project_slug);
              busySlugs = next;
              batchLine = message;
            }
          },
        ),
      );
      unsubs.push(
        await listen<BatchUpdateResult>("dashboard-batch-done", () => {
          busySlugs = new Set();
          batchLine = "";
          void loadSummary();
        }),
      );
    })();
    return () => unsubs.forEach((u) => u());
  });

  $effect(() => {
    void loadSummary();
  });
</script>

<svelte:window onclick={onWindowClick} />

<div class="flex min-h-0 flex-1 flex-col gap-4 p-4">
  <header class="shrink-0">
    <h2 class="text-lg font-semibold text-tr-ink">{tr("dashboard.title")}</h2>
    <p class="mt-1 text-sm text-tr-ink-3">{tr("dashboard.descLine1")}</p>
  </header>

  {#if !summary}
    <div class="flex items-center gap-2 text-sm text-tr-ink-3" role="status" aria-busy="true">
      <Loader2 size={16} class="animate-spin" aria-hidden="true" />
      <span>{tr("dashboard.refresh")}</span>
    </div>
  {:else if rows.length === 0}
    <div class="flex min-h-40 flex-1 items-center justify-center">
      <div
        class="flex w-full max-w-sm flex-col items-center gap-3 rounded-xl border border-tr-border bg-tr-surface px-6 py-10 text-center"
      >
        <Gauge size={28} class="text-tr-ink-4" aria-hidden="true" />
        <p class="text-sm font-medium text-tr-ink">{tr("dashboard.empty.title")}</p>
        <p class="text-xs text-tr-ink-3">{tr("dashboard.empty.hint")}</p>
        <button
          type="button"
          class="tr-press mt-1 inline-flex items-center gap-1.5 rounded-lg bg-tr-accent px-3.5 py-1.5 text-xs font-medium text-tr-on-accent transition-colors hover:bg-tr-accent-hover disabled:opacity-50"
          disabled={adding}
          onclick={() => void addProjectsFlow()}
        >
          {#if adding}
            <Loader2 size={14} class="animate-spin" aria-hidden="true" />
            {tr("dashboard.actions.adding")}
          {:else}
            <Plus size={14} aria-hidden="true" />
            {tr("dashboard.actions.add")}
          {/if}
        </button>
      </div>
    </div>
  {:else}
    {#if !summary.binary_available}
      <div
        class="shrink-0 rounded-xl border border-tr-watch/30 bg-tr-watch-soft px-4 py-3"
        role="note"
      >
        <div class="flex items-start gap-3">
          <TriangleAlert size={18} class="mt-0.5 shrink-0 text-tr-watch" aria-hidden="true" />
          <p class="text-sm text-tr-on-watch">{tr("dashboard.binary.missing")}</p>
        </div>
      </div>
    {/if}

    <div class="grid shrink-0 grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
      {#each metricCards as card (card.key)}
        <div class="rounded-xl border border-tr-border bg-tr-surface px-4 py-3">
          <p class="text-[11px] font-medium uppercase tracking-wide text-tr-ink-4">
            {tr(`dashboard.totals.${card.key}`)}
          </p>
          <p class="mt-1 text-xl font-semibold text-tr-ink">{card.value}</p>
        </div>
      {/each}
    </div>

    <div class="flex shrink-0 flex-wrap items-center gap-2">
      <button
        type="button"
        class="tr-press inline-flex items-center gap-1.5 rounded-lg bg-tr-accent px-3.5 py-1.5 text-xs font-medium text-tr-on-accent transition-colors hover:bg-tr-accent-hover disabled:opacity-50"
        disabled={batchRunning || !summary.binary_available}
        onclick={() => void runBatch()}
      >
        {#if batchRunning}
          <Loader2 size={14} class="animate-spin" aria-hidden="true" />
          {tr("dashboard.actions.batching")}
        {:else}
          <RefreshCw size={14} aria-hidden="true" />
          {tr("dashboard.actions.batch")}
        {/if}
      </button>
      <button
        type="button"
        class="tr-press inline-flex items-center gap-1.5 rounded-lg border border-tr-border-strong px-3.5 py-1.5 text-xs transition-colors hover:bg-tr-elevated"
        disabled={adding}
        onclick={() => void addProjectsFlow()}
      >
        {#if adding}
          <Loader2 size={14} class="animate-spin" aria-hidden="true" />
          {tr("dashboard.actions.adding")}
        {:else}
          <Plus size={14} aria-hidden="true" />
          {tr("dashboard.actions.add")}
        {/if}
      </button>
      <button
        type="button"
        class="tr-press inline-flex items-center gap-1.5 rounded-lg border border-tr-border-strong px-3.5 py-1.5 text-xs transition-colors hover:bg-tr-elevated"
        onclick={() => void loadSummary()}
      >
        <RefreshCw size={14} aria-hidden="true" />
        {tr("dashboard.refresh")}
      </button>
    </div>

    {#if summary.vault_root_exists === false}
      <div class="shrink-0 rounded-xl border border-tr-watch/30 bg-tr-watch-soft px-4 py-3" role="note">
        <div class="flex items-start gap-3">
          <TriangleAlert size={18} class="mt-0.5 shrink-0 text-tr-watch" aria-hidden="true" />
          <div class="text-sm text-tr-on-watch">
            <p>{tr("dashboard.vault.banner")}</p>
            <p class="mt-1 text-xs opacity-80">{tr("dashboard.vault.bannerHint")}</p>
          </div>
        </div>
      </div>
    {/if}

    {#if batchRunning && batchLine}
      <div
        class="shrink-0 rounded-lg border border-tr-accent-soft bg-tr-accent-soft px-3 py-2"
        role="status"
        aria-live="polite"
      >
        <p class="truncate text-xs text-tr-ink-2">{batchLine}</p>
      </div>
    {/if}

    <div class="min-h-0 flex-1 overflow-y-auto rounded-xl border border-tr-border">
      <table class="w-full border-collapse text-left text-xs">
        <thead class="sticky top-0 bg-tr-surface text-[11px] uppercase tracking-wide text-tr-ink-4">
          <tr>
            <th class="px-3 py-2 font-medium">{tr("dashboard.table.project")}</th>
            <th class="px-3 py-2 font-medium">{tr("dashboard.table.status")}</th>
            <th class="px-3 py-2 font-medium">{tr("dashboard.table.graph")}</th>
            <th class="px-3 py-2 font-medium">{tr("dashboard.table.staleness")}</th>
            <th class="px-3 py-2 font-medium">{tr("dashboard.table.lastBatch")}</th>
            <th class="px-3 py-2 font-medium"><span class="sr-only">{tr("dashboard.actions.updateRow")}</span></th>
          </tr>
        </thead>
        <tbody>
          {#each rows as row (row.slug)}
            <tr class="border-t border-tr-border">
              <td class="max-w-56 px-3 py-2">
                <div class="font-medium text-tr-ink">{row.name}</div>
                <div class="truncate text-[11px] text-tr-ink-4" title={row.repo_path}>{row.repo_path}</div>
              </td>
              <td class="px-3 py-2">
                <span class={`inline-block rounded-full px-2.5 py-1 ${statusChipClass(row.status)}`}>
                  {statusLabel(row.status)}
                </span>
                {#if row.docs.vault === false && summary?.vault_root_exists && row.graph.graph_present}
                  <span
                    class="ml-1 inline-flex items-center gap-0.5 text-[11px] text-tr-ink-4"
                    title={tr("dashboard.vault.rowHint")}
                  >
                    <TriangleAlert size={10} aria-hidden="true" />
                    {tr("dashboard.docs.vault")}
                  </span>
                {/if}
                {#if row.status === "partial" && row.missing_assets.length > 0}
                  <p class="mt-1 text-[11px] text-tr-on-watch">
                    {tr("dashboard.table.missingAssets.count", { count: row.missing_assets.length })}
                  </p>
                {/if}
              </td>
              <td class="px-3 py-2 text-tr-ink-2">
                {#if busySlugs.has(row.slug)}
                  <Loader2 size={14} class="animate-spin text-tr-accent" aria-hidden="true" />
                {:else}
                  {graphCell(row)}
                {/if}
              </td>
              <td class="px-3 py-2">
                <span class={`inline-block rounded-full px-2.5 py-1 ${stalenessChipClass(row)}`}>
                  {stalenessLabel(row)}
                </span>
              </td>
              <td class="px-3 py-2 text-tr-ink-3">{lastBatchCell(row)}</td>
              <td class="relative px-3 py-2 align-middle whitespace-nowrap">
                <button
                  type="button"
                  class="tr-press inline-flex items-center justify-center rounded-md border border-tr-border p-1.5 text-tr-ink-2 transition-colors hover:bg-tr-elevated"
                  aria-haspopup="menu"
                  aria-expanded={menuOpenSlug === row.slug}
                  aria-label={tr("dashboard.menu.label")}
                  data-kebab
                  onclick={() => toggleMenu(row.slug)}
                >
                  <EllipsisVertical size={14} aria-hidden="true" />
                </button>
                {#if menuOpenSlug === row.slug}
                  <div
                    role="menu"
                    data-menu-panel
                    class="absolute right-0 top-full z-10 mt-1 w-48 overflow-hidden rounded-lg border border-tr-border bg-tr-surface shadow-lg"
                  >
                    <button
                      type="button"
                      role="menuitem"
                      class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs text-tr-ink-2 transition-colors hover:bg-tr-elevated disabled:cursor-not-allowed disabled:opacity-40"
                      disabled={batchRunning || busySlugs.has(row.slug)}
                      onclick={() => void updateRowGraph(row)}
                    >
                      <RefreshCw size={12} aria-hidden="true" />
                      {tr("dashboard.menu.update")}
                    </button>
                    <button
                      type="button"
                      role="menuitem"
                      class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs text-tr-ink-2 transition-colors hover:bg-tr-elevated"
                      onclick={() => openRowFolder(row)}
                    >
                      <FolderOpen size={12} aria-hidden="true" />
                      {tr("dashboard.menu.open")}
                    </button>
                    <button
                      type="button"
                      role="menuitem"
                      class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs text-tr-critical transition-colors hover:bg-tr-elevated"
                      onclick={() => void deleteProject(row)}
                    >
                      <TriangleAlert size={12} aria-hidden="true" />
                      {tr("dashboard.menu.remove")}
                    </button>
                  </div>
                {/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    {#if summary && rows.length > 0}
      <section class="shrink-0 rounded-xl border border-tr-border bg-tr-surface p-4">
        <h3 class="text-sm font-medium text-tr-ink">{tr("dashboard.charts.commits")}</h3>
        <div class="mt-3 grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
          {#each rows as row (row.slug)}
            {@const max = weeklyMax.get(row.slug) ?? 1}
            <div class="rounded-lg border border-tr-border px-3 py-2">
              <div class="flex items-baseline justify-between gap-2">
                <span class="truncate text-xs text-tr-ink-2">{row.name}</span>
                <span class="shrink-0 text-xs text-tr-ink-4">
                  {tr("dashboard.charts.commitsTotal", { count: Number(row.activity.total_commits) })}
                </span>
              </div>
              <svg
                viewBox="0 0 264 40"
                preserveAspectRatio="none"
                class="mt-1 block h-10 w-full"
                role="img"
                aria-label={tr("dashboard.charts.commitsTotal", { count: Number(row.activity.total_commits) })}
              >
                {#each row.activity.weekly_commits as wk, i (i)}
                  {@const v = Number(wk)}
                  {@const h = v > 0 ? Math.max(2, (v / max) * 38) : 2}
                  <rect
                    x={i * 22}
                    y={40 - h}
                    width={20}
                    height={h}
                    rx="1"
                    fill="var(--color-tr-accent)"
                  >
                    <title>{11 - i}w: {v}</title>
                  </rect>
                {/each}
              </svg>
            </div>
          {/each}
        </div>
      </section>

      <section class="shrink-0 rounded-xl border border-tr-border bg-tr-surface p-4">
        <h3 class="text-sm font-medium text-tr-ink">{tr("dashboard.charts.collab")}</h3>
        <p class="mt-0.5 text-xs text-tr-ink-3">{tr("dashboard.charts.collabHint")}</p>
        {#if collaborations.length === 0}
          <p class="mt-2 text-xs text-tr-ink-4">{tr("dashboard.charts.noCollab")}</p>
        {:else}
          <div class="mt-2 flex flex-col">
            {#each collaborations as c (c.project_a + c.project_b)}
              {@const shared = Number(c.shared_authors)}
              <div class="flex items-center gap-3 py-1.5">
                <span class="w-48 shrink-0 truncate text-xs text-tr-ink-2">
                  {c.project_a} ↔ {c.project_b}
                </span>
                <div class="h-1.5 min-w-0 flex-1 rounded-full bg-tr-elevated">
                  <div
                    class="h-full rounded-full bg-tr-accent"
                    style={`width:${maxShared > 0 ? (shared / maxShared) * 100 : 0}%`}
                  ></div>
                </div>
                <span class="shrink-0 whitespace-nowrap text-xs text-tr-ink-3">
                  {tr("dashboard.charts.sharedAuthors", { count: shared })}
                </span>
              </div>
            {/each}
          </div>
        {/if}
      </section>

      <section class="shrink-0 rounded-xl border border-tr-border bg-tr-surface p-4">
        <h3 class="text-sm font-medium text-tr-ink">{tr("dashboard.charts.coverage")}</h3>
        <div class="mt-2 mb-2 flex flex-wrap gap-x-4 gap-y-1">
          {#each docsLegend as item (item.key)}
            <span class="inline-flex items-center gap-1.5">
              <span class="h-[3px] w-4 rounded-full bg-tr-border-strong" aria-hidden="true"></span>
              <span class="text-[11px] text-tr-ink-3">{tr(`dashboard.docs.${item.key}`)}</span>
            </span>
          {/each}
        </div>
        <div class="flex flex-col">
          {#each rows as row (row.slug)}
            <div class="flex items-center gap-3 py-1.5">
              <span class="w-48 shrink-0 truncate text-xs text-tr-ink-2">{row.name}</span>
              <div class="flex min-w-0 flex-1 gap-[2px]">
                {#each docsLegend as seg (seg.key)}
                  {@const ok = row.docs[seg.field]}
                  <div
                    class="h-1.5 flex-1 rounded-[2px]"
                    style={`background:${ok ? "var(--color-tr-good)" : "var(--color-tr-border-strong)"}`}
                    title={`${tr(`dashboard.docs.${seg.key}`)}: ${tr(ok ? "dashboard.docs.present" : "dashboard.docs.missing")}`}
                  ></div>
                {/each}
              </div>
              <span class="shrink-0 text-[11px] text-tr-ink-4">{docPresent(row)}/6</span>
            </div>
          {/each}
        </div>
      </section>
    {/if}
  {/if}
</div>