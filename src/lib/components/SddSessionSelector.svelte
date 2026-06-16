<script lang="ts">
  import type { SddSessionInfo } from "../types";

  interface Props {
    sessions: SddSessionInfo[];
    activeSessionId?: string | null;
    open: boolean;
    creating?: boolean;
    ontoggle: () => void;
    onselect: (sessionId: string) => void;
    oncreate: (title: string) => void;
    ondelete: (sessionId: string) => void;
  }

  let {
    sessions,
    activeSessionId = null,
    open,
    creating = false,
    ontoggle,
    onselect,
    oncreate,
    ondelete,
  }: Props = $props();

  const active = $derived(sessions.find((s) => s.id === activeSessionId) ?? null);

  let triggerEl = $state<HTMLButtonElement | null>(null);
  let menuTop = $state(0);
  let menuLeft = $state(0);
  let menuWidth = $state(320);
  let newTitle = $state("");
  let showCreate = $state(false);

  function updateMenuPosition() {
    if (!triggerEl) return;
    const rect = triggerEl.getBoundingClientRect();
    menuTop = rect.bottom + 4;
    menuLeft = rect.left;
    menuWidth = Math.max(rect.width, 300);
  }

  function handleCreate() {
    const title = newTitle.trim() || "新需求";
    oncreate(title);
    newTitle = "";
    showCreate = false;
  }

  function confirmDelete(session: SddSessionInfo, e: MouseEvent) {
    e.stopPropagation();
    if (
      !confirm(
        `删除需求「${session.title}」？\n\n将永久删除该 SDD 任务及 ~/.mind-mesh/sdd/ 下的全部产出，不可恢复。`,
      )
    ) {
      return;
    }
    ondelete(session.id);
  }

  $effect(() => {
    if (!open) {
      showCreate = false;
      return;
    }
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

<div class="relative mb-5">
  <button
    type="button"
    bind:this={triggerEl}
    class="flex w-full items-center gap-2 rounded-xl border border-white/10 bg-white/[0.03] px-3 py-2.5 text-left text-sm hover:bg-white/[0.06]"
    onclick={ontoggle}
    aria-expanded={open}
    aria-haspopup="listbox"
  >
    <span class="min-w-0 flex-1">
      <span class="block text-[10px] text-white/40">当前 SDD 需求</span>
      <span class="block truncate font-medium text-white/90">
        {active?.title ?? "选择或新建需求…"}
      </span>
    </span>
    <span class="shrink-0 text-white/40">{open ? "▴" : "▾"}</span>
  </button>

  {#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="fixed inset-0 z-[200]" onclick={ontoggle} role="presentation"></div>
    <div
      class="fixed z-[201] overflow-hidden rounded-xl border border-white/10 bg-[#1a1e26] shadow-xl"
      style="top: {menuTop}px; left: {menuLeft}px; width: {menuWidth}px;"
      role="listbox"
    >
      <div class="border-b border-white/10 px-3 py-2 text-xs text-white/45">
        {sessions.length} 个并行需求 · 本地存储，不入 Git
      </div>
      <ul class="max-h-56 overflow-y-auto py-1">
        {#each sessions as session}
          <li class="mx-1 flex items-stretch gap-0.5">
            <button
              type="button"
              class={`flex min-w-0 flex-1 flex-col rounded-lg px-3 py-2 text-left text-sm hover:bg-white/5 ${
                activeSessionId === session.id ? "bg-indigo-500/15 text-indigo-100" : ""
              }`}
              onclick={() => onselect(session.id)}
              role="option"
              aria-selected={activeSessionId === session.id}
            >
              <span class="truncate font-medium">{session.title}</span>
              {#if session.created_at}
                <span class="text-[10px] text-white/35">{session.created_at}</span>
              {/if}
            </button>
            <button
              type="button"
              class="shrink-0 rounded-lg px-2 text-white/35 hover:bg-red-500/10 hover:text-red-300"
              title="删除此需求及全部产出"
              aria-label={`删除 ${session.title}`}
              onclick={(e) => confirmDelete(session, e)}
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                <path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/>
              </svg>
            </button>
          </li>
        {:else}
          <li class="px-3 py-3 text-xs text-white/40">暂无需求，请在下方新建</li>
        {/each}
      </ul>
      <div class="border-t border-white/10 p-2">
        {#if showCreate}
          <div class="flex gap-2">
            <input
              class="min-w-0 flex-1 rounded-lg border border-white/10 bg-black/25 px-2.5 py-1.5 text-xs outline-none focus:border-indigo-500"
              placeholder="需求标题…"
              bind:value={newTitle}
              disabled={creating}
              onkeydown={(e) => e.key === "Enter" && handleCreate()}
            />
            <button
              type="button"
              class="shrink-0 rounded-lg bg-indigo-600 px-3 py-1.5 text-xs font-medium hover:bg-indigo-500 disabled:opacity-40"
              disabled={creating}
              onclick={handleCreate}
            >
              {creating ? "创建中…" : "创建"}
            </button>
          </div>
        {:else}
          <button
            type="button"
            class="w-full rounded-lg border border-dashed border-white/15 px-3 py-2 text-xs text-indigo-300 hover:border-indigo-500/40 hover:bg-indigo-500/5"
            onclick={() => (showCreate = true)}
          >
            + 新建 SDD 需求
          </button>
        {/if}
      </div>
    </div>
  {/if}
</div>
