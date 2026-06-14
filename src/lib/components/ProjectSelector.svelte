<script lang="ts">
  import type { ProjectSummary } from "../types";

  interface Props {
    projects: ProjectSummary[];
    selectedSlug: string | null;
    open: boolean;
    addBusy?: boolean;
    ontoggle: () => void;
    onselect: (project: ProjectSummary) => void;
    onadd: () => void;
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
</script>

<div class="relative">
  <button
    type="button"
    bind:this={triggerEl}
    class="flex max-w-[220px] items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-3 py-1.5 text-left text-sm hover:bg-white/10"
    onclick={ontoggle}
    aria-expanded={open}
    aria-haspopup="listbox"
  >
    <span class="min-w-0 flex-1 truncate font-medium">
      {selected?.name ?? "Select project"}
    </span>
    <span class="shrink-0 text-white/40">{open ? "▴" : "▾"}</span>
  </button>

  {#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div
      class="fixed inset-0 z-[200]"
      onclick={ontoggle}
      role="presentation"
    ></div>
    <div
      class="fixed z-[201] w-80 overflow-hidden rounded-lg border border-white/10 bg-[#1a1e26] shadow-xl"
      style="top: {menuTop}px; left: {menuLeft}px;"
      role="listbox"
    >
      <div class="border-b border-white/10 px-3 py-2 text-xs text-white/40">
        {projects.length} project{projects.length === 1 ? "" : "s"}
      </div>
      <ul class="max-h-64 overflow-y-auto py-1">
        {#each projects as project}
          <li class="mx-1 flex items-stretch gap-1">
            <button
              type="button"
              class={`flex min-w-0 flex-1 flex-col rounded px-3 py-2 text-left text-sm hover:bg-white/5 ${
                selectedSlug === project.slug ? "bg-indigo-500/15 text-indigo-100" : ""
              }`}
              onclick={() => onselect(project)}
              role="option"
              aria-selected={selectedSlug === project.slug}
            >
              <span class="font-medium">{project.name}</span>
              <span class="truncate text-xs text-white/40">{project.slug}</span>
            </button>
            {#if project.repo_path && onopenFolder}
              <button
                type="button"
                class="shrink-0 rounded px-2 text-white/40 hover:bg-white/5 hover:text-white/80"
                title="Open repository folder"
                aria-label={`Open folder for ${project.name}`}
                onclick={(e) => {
                  e.stopPropagation();
                  onopenFolder(project);
                }}
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                  <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>
                </svg>
              </button>
            {/if}
          </li>
        {:else}
          <li class="px-3 py-4 text-sm text-white/40">尚无项目，请添加仓库。</li>
        {/each}
      </ul>
      <div class="border-t border-white/10 p-2">
        <button
          type="button"
          class="w-full rounded-lg bg-indigo-600 px-3 py-2 text-sm font-medium hover:bg-indigo-500 disabled:opacity-50"
          disabled={addBusy}
          onclick={onadd}
        >
          {addBusy ? "正在添加并初始化…" : "+ 添加并初始化仓库"}
        </button>
      </div>
    </div>
  {/if}
</div>
