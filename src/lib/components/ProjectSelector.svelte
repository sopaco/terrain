<script lang="ts">
  import { Folder, Plus, X } from "@lucide/svelte";
  import type { ProjectSummary } from "../types";
  import { UI_MESSAGES } from "../terminology";
  import ChevronIcon from "./icons/ChevronIcon.svelte";

  interface Props {
    projects: ProjectSummary[];
    selectedSlug: string | null;
    open: boolean;
    addBusy?: boolean;
    ontoggle: () => void;
    onselect: (project: ProjectSummary) => void;
    onadd: () => void;
    onremove?: (project: ProjectSummary) => void;
    onopenFolder?: (project: ProjectSummary) => void;
  }

  let {
    projects,
    selectedSlug,
    open,
    addBusy = false,
    ontoggle,
    onselect,
    onadd,
    onremove,
    onopenFolder,
  }: Props = $props();

  const selected = $derived(projects.find((p) => p.slug === selectedSlug) ?? null);

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
  function confirmRemove(project: ProjectSummary, e: MouseEvent) {
    e.stopPropagation();
    if (
      !confirm(
        `从列表中移除「${project.name}」？\n\n仅移除 Terrain 登记，不会删除仓库或 .terrain/ 知识资产。`,
      )
    ) {
      return;
    }
    onremove?.(project);
  }
</script>

<div class="relative">
  <button
    type="button"
    bind:this={triggerEl}
    class="flex max-w-[220px] items-center gap-2 rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-1.5 text-left text-sm hover:bg-tr-elevated"
    onclick={ontoggle}
    aria-expanded={open}
    aria-haspopup="listbox"
  >
    <span class="min-w-0 flex-1 truncate font-medium">
      {selected?.name ?? UI_MESSAGES.selectProject}
    </span>
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
      class="fixed z-[201] w-80 overflow-hidden rounded-lg border border-tr-border-strong bg-tr-raised shadow-xl"
      style="top: {menuTop}px; left: {menuLeft}px;"
      role="listbox"
    >
      <ul class="max-h-64 overflow-y-auto py-1">
        {#each projects as project}
          <li class="mx-1 flex items-stretch gap-1">
            <button
              type="button"
              class={`flex min-w-0 flex-1 items-center rounded px-3 py-2 text-left text-sm hover:bg-tr-elevated ${
                selectedSlug === project.slug ? "bg-tr-accent-soft-strong text-tr-on-accent" : ""
              }`}
              onclick={() => onselect(project)}
              role="option"
              aria-selected={selectedSlug === project.slug}
            >
              <span class="truncate font-medium">{project.name}</span>
            </button>
            {#if project.repo_path && onopenFolder}
              <button
                type="button"
                class="inline-flex shrink-0 items-center justify-center rounded px-2 text-tr-ink-3 hover:bg-tr-elevated hover:text-tr-ink-2"
                title="打开仓库目录"
                aria-label={`Open folder for ${project.name}`}
                onclick={(e) => {
                  e.stopPropagation();
                  onopenFolder(project);
                }}
              >
                <Folder size={16} strokeWidth={2} aria-hidden="true" />
              </button>
            {/if}
            {#if onremove}
              <button
                type="button"
                class="inline-flex shrink-0 items-center justify-center rounded px-2 text-tr-ink-3 hover:bg-tr-critical-soft hover:text-tr-critical"
                title="从列表移除"
                aria-label={`从列表移除 ${project.name}`}
                onclick={(e) => confirmRemove(project, e)}
              >
                <X size={14} strokeWidth={2} aria-hidden="true" />
              </button>
            {/if}
          </li>
        {:else}
          <li class="px-3 py-4 text-sm text-tr-ink-3">尚无项目，请添加仓库。</li>
        {/each}
      </ul>
      <div class="border-t border-tr-border-strong p-2">
        <button
          type="button"
          class="inline-flex w-full items-center justify-center gap-1.5 rounded-lg bg-tr-accent px-3 py-2 text-sm font-medium hover:bg-tr-accent-hover disabled:opacity-50"
          disabled={addBusy}
          onclick={onadd}
        >
          <Plus size={14} strokeWidth={2} aria-hidden="true" />
          {addBusy ? "正在添加并初始化…" : "添加并初始化仓库"}
        </button>
      </div>
    </div>
  {/if}
</div>
