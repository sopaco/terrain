<script lang="ts">
  import { Folder, Plus, X } from "@lucide/svelte";
  import type { ProjectRegistryEntry } from "../types";
  import {
    registryDisplayName,
    selectedProjectDisplayName,
    statusBadgeLabel,
    statusBadgeTitle,
  } from "../projectRegistry";
  import { tr } from "../i18n";
  import ChevronIcon from "./icons/ChevronIcon.svelte";

  interface Props {
    registryProjects: ProjectRegistryEntry[];
    selectedSlug: string | null;
    open: boolean;
    addBusy?: boolean;
    /** Display name of the project currently being imported (may not be registered yet). */
    importingName?: string | null;
    importingPath?: string | null;
    importingSlug?: string | null;
    ontoggle: () => void;
    onselect: (project: ProjectRegistryEntry) => void;
    onadd: () => void;
    onremove?: (project: ProjectRegistryEntry) => void;
    onopenFolder?: (project: ProjectRegistryEntry) => void;
  }

  let {
    registryProjects,
    selectedSlug,
    open,
    addBusy = false,
    importingName = null,
    importingPath = null,
    importingSlug = null,
    ontoggle,
    onselect,
    onadd,
    onremove,
    onopenFolder,
  }: Props = $props();

  const displayName = $derived(
    selectedProjectDisplayName(
      selectedSlug,
      registryProjects,
      tr("terms.msg.selectProject"),
    ),
  );

  const selectedEntry = $derived(
    registryProjects.find((p) => p.slug === selectedSlug) ?? null,
  );

  const selectedImporting = $derived(
    selectedEntry ? isImportTarget(selectedEntry) : false,
  );

  const selectedBadge = $derived(
    selectedImporting
      ? tr("misc.projects.importing")
      : selectedEntry
        ? statusBadgeLabel(selectedEntry.status)
        : null,
  );

  const selectedBadgeTitle = $derived(
    selectedImporting
      ? tr("misc.projects.importingTitle")
      : selectedEntry
        ? statusBadgeTitle(selectedEntry)
        : undefined,
  );

  /** The import target only joins the list after the backend registers it. */
  const importingInList = $derived(
    importingSlug
      ? registryProjects.some((p) => p.slug === importingSlug)
      : importingPath
        ? registryProjects.some((p) => p.repo_path === importingPath)
        : false,
  );

  const showPendingImport = $derived(
    Boolean(importingName && !importingInList),
  );

  function isImportTarget(entry: ProjectRegistryEntry): boolean {
    if (!importingName) return false;
    if (importingSlug && entry.slug === importingSlug) return true;
    if (importingPath && entry.repo_path === importingPath) return true;
    return false;
  }

  let triggerEl = $state<HTMLButtonElement | null>(null);
  let menuTop = $state(0);
  let menuLeft = $state(0);

  function updateMenuPosition() {
    if (!triggerEl) return;
    const rect = triggerEl.getBoundingClientRect();
    menuTop = rect.bottom + 4;
    menuLeft = rect.left;
  }

  $effect(() => {
    if (!open) return;
    updateMenuPosition();
    const onLayout = () => updateMenuPosition();
    window.addEventListener("resize", onLayout);
    window.addEventListener("scroll", onLayout, true);
    return () => {
      window.removeEventListener("resize", onLayout);
      window.removeEventListener("scroll", onLayout, true);
    };
  });

  function confirmRemove(entry: ProjectRegistryEntry, e: MouseEvent) {
    e.stopPropagation();
    // The running import would re-register the project at the end, so removing
    // it mid-flight only looks like it worked.
    if (isImportTarget(entry)) return;
    const label = registryDisplayName(entry);
    if (!confirm(tr("misc.projects.removeConfirm", { label }))) {
      return;
    }
    onremove?.(entry);
  }
</script>

<div class="relative">
  <button
    type="button"
    bind:this={triggerEl}
    class="tr-press flex max-w-[240px] items-center gap-2 rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-1.5 text-left text-sm transition-colors hover:bg-tr-elevated"
    onclick={ontoggle}
    aria-expanded={open}
    aria-haspopup="listbox"
  >
    <span class="min-w-0 flex-1 truncate font-medium">
      {displayName}
    </span>
    {#if selectedBadge}
      <span
        class={`shrink-0 rounded-full px-1.5 py-0.5 text-[10px] font-medium ${
          selectedImporting
            ? "bg-tr-watch text-tr-on-watch"
            : "bg-tr-watch-soft text-tr-watch"
        }`}
        title={selectedBadgeTitle}>{selectedBadge}</span
      >
    {/if}
    <ChevronIcon direction={open ? "up" : "down"} size={14} />
  </button>

  {#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-[200]"
      onclick={ontoggle}
      role="presentation"
    ></div>
    <div
      class="tr-pop fixed z-[201] w-80 overflow-hidden rounded-lg border border-tr-border-strong bg-tr-raised shadow-xl"
      style="top: {menuTop}px; left: {menuLeft}px; --tr-pop-origin: top left;"
      role="listbox"
    >
      <ul class="max-h-64 overflow-y-auto py-1">
        {#if showPendingImport}
          <li
            class="mx-1 mb-1 flex items-start gap-2 rounded-lg border border-dashed border-tr-watch/40 bg-tr-watch-soft px-3 py-2"
            aria-live="polite"
          >
            <span
              class="mt-0.5 inline-block h-3.5 w-3.5 shrink-0 animate-spin rounded-full border-2 border-tr-watch border-t-transparent"
              aria-hidden="true"
            ></span>
            <div class="min-w-0 flex-1">
              <p class="truncate text-sm font-medium text-tr-ink" title={importingName ?? ""}>
                {importingName}
              </p>
              {#if importingPath}
                <p
                  class="mt-0.5 truncate font-mono text-[10px] text-tr-ink-3"
                  title={importingPath}
                >
                  {importingPath}
                </p>
              {/if}
            </div>
            <span
              class="shrink-0 rounded-full bg-tr-watch px-1.5 py-0.5 text-[10px] font-medium text-tr-on-watch"
            >
              {tr("misc.projects.importing")}
            </span>
          </li>
        {/if}
        {#each registryProjects as entry (entry.slug)}
          {@const importing = isImportTarget(entry)}
          {@const badge = importing
            ? tr("misc.projects.importing")
            : statusBadgeLabel(entry.status)}
          {@const badgeTitle = importing
            ? tr("misc.projects.importingTitle")
            : statusBadgeTitle(entry)}
          <li class="mx-1 flex items-stretch gap-1">
            <button
              type="button"
              class={`flex min-w-0 flex-1 flex-col rounded px-3 py-2 text-left text-sm transition-colors hover:bg-tr-elevated ${
                selectedSlug === entry.slug
                  ? "bg-tr-accent-soft-strong text-tr-on-accent"
                  : ""
              }`}
              onclick={() => onselect(entry)}
              role="option"
              aria-selected={selectedSlug === entry.slug}
            >
              <span class="flex min-w-0 items-center gap-1.5">
                <span class="truncate font-medium"
                  >{registryDisplayName(entry)}</span
                >
                {#if badge}
                  <span
                    class="shrink-0 rounded-full px-1.5 py-0.5 text-[10px] font-medium ${
                      importing
                        ? "bg-tr-watch text-tr-on-watch"
                        : "bg-tr-watch-soft text-tr-watch"
                    }"
                    title={badgeTitle}>{badge}</span
                  >
                {/if}
              </span>
              {#if entry.status !== "ready"}
                <span
                  class="mt-0.5 truncate font-mono text-[10px] text-tr-ink-3"
                  title={entry.repo_path}
                >
                  {entry.repo_path}
                </span>
              {/if}
            </button>
            {#if onopenFolder}
              <button
                type="button"
                class="tr-press inline-flex shrink-0 items-center justify-center rounded px-2 text-tr-ink-3 transition-colors hover:bg-tr-elevated hover:text-tr-ink-2"
                title={tr("misc.projects.openFolder")}
                aria-label={tr("misc.projects.openFolderFor", { name: registryDisplayName(entry) })}
                onclick={(e) => {
                  e.stopPropagation();
                  onopenFolder(entry);
                }}
              >
                <Folder size={16} strokeWidth={2} aria-hidden="true" />
              </button>
            {/if}
            {#if onremove}
              <button
                type="button"
                class="tr-press inline-flex shrink-0 items-center justify-center rounded px-2 text-tr-ink-3 transition-colors hover:bg-tr-critical-soft hover:text-tr-critical disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-transparent disabled:hover:text-tr-ink-3"
                title={importing
                  ? tr("misc.projects.removeBusy")
                  : tr("misc.projects.remove")}
                aria-label={tr("misc.projects.removeAria", { label: registryDisplayName(entry) })}
                disabled={importing}
                onclick={(e) => confirmRemove(entry, e)}
              >
                <X size={14} strokeWidth={2} aria-hidden="true" />
              </button>
            {/if}
          </li>
        {:else}
          {#if !showPendingImport}
            <li class="px-3 py-4 text-sm text-tr-ink-3">{tr("misc.projects.empty")}</li>
          {/if}
        {/each}
      </ul>
      <div class="border-t border-tr-border-strong p-2">
        <button
          type="button"
          class="tr-press inline-flex w-full items-center justify-center gap-1.5 rounded-lg bg-tr-accent px-3 py-2 text-sm font-medium transition-colors hover:bg-tr-accent-hover disabled:opacity-50"
          disabled={addBusy}
          onclick={onadd}
        >
          <Plus size={14} strokeWidth={2} aria-hidden="true" />
          {addBusy ? tr("misc.projects.adding") : tr("misc.projects.add")}
        </button>
      </div>
    </div>
  {/if}
</div>
