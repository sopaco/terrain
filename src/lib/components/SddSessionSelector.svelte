<script lang="ts">
  import { Plus, Trash2 } from "@lucide/svelte";
  import type { SddSessionInfo } from "../types";
  import ChevronIcon from "./icons/ChevronIcon.svelte";

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
        `删除需求「${session.title}」？\n\n将永久删除该 SDD 任务及 ~/.terrain/sdd/ 下的全部产出，不可恢复。`,
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
    class="flex w-full items-center gap-2 rounded-xl border border-tr-border-strong bg-tr-elevated px-3 py-2.5 text-left text-sm hover:bg-tr-raised"
    onclick={ontoggle}
    aria-expanded={open}
    aria-haspopup="listbox"
  >
    <span class="min-w-0 flex-1">
      <span class="block text-[10px] text-tr-ink-3">当前 SDD 需求</span>
      <span class="block truncate font-medium text-tr-ink">
        {active?.title ?? "选择或新建需求…"}
      </span>
    </span>
    <ChevronIcon direction={open ? "up" : "down"} size={14} />
  </button>

  {#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="fixed inset-0 z-[200]" onclick={ontoggle} role="presentation"></div>
    <div
      class="fixed z-[201] overflow-hidden rounded-xl border border-tr-border-strong bg-tr-raised shadow-xl"
      style="top: {menuTop}px; left: {menuLeft}px; width: {menuWidth}px;"
      role="listbox"
    >
      <div class="border-b border-tr-border-strong px-3 py-2 text-xs text-tr-ink-3">
        {sessions.length} 个并行需求 · 本地存储，不入 Git
      </div>
      <ul class="max-h-56 overflow-y-auto py-1">
        {#each sessions as session}
          <li class="mx-1 flex items-stretch gap-0.5">
            <button
              type="button"
              class={`flex min-w-0 flex-1 flex-col rounded-lg px-3 py-2 text-left text-sm hover:bg-tr-elevated ${
                activeSessionId === session.id ? "bg-tr-accent-soft-strong text-tr-on-accent" : ""
              }`}
              onclick={() => onselect(session.id)}
              role="option"
              aria-selected={activeSessionId === session.id}
            >
              <span class="truncate font-medium">{session.title}</span>
              {#if session.created_at}
                <span class="text-[10px] text-tr-ink-3">{session.created_at}</span>
              {/if}
            </button>
            <button
              type="button"
              class="inline-flex shrink-0 items-center justify-center rounded-lg px-2 text-tr-ink-3 hover:bg-tr-critical-soft hover:text-tr-critical"
              title="删除此需求及全部产出"
              aria-label={`删除 ${session.title}`}
              onclick={(e) => confirmDelete(session, e)}
            >
              <Trash2 size={14} strokeWidth={2} aria-hidden="true" />
            </button>
          </li>
        {:else}
          <li class="px-3 py-3 text-xs text-tr-ink-3">暂无需求，请在下方新建</li>
        {/each}
      </ul>
      <div class="border-t border-tr-border-strong p-2">
        {#if showCreate}
          <div class="flex gap-2">
            <input
              class="min-w-0 flex-1 rounded-lg border border-tr-border-strong bg-tr-page px-2.5 py-1.5 text-xs outline-none focus:border-tr-accent"
              placeholder="需求标题…"
              bind:value={newTitle}
              disabled={creating}
              onkeydown={(e) => e.key === "Enter" && handleCreate()}
            />
            <button
              type="button"
              class="shrink-0 rounded-lg bg-tr-accent px-3 py-1.5 text-xs font-medium hover:bg-tr-accent-hover disabled:opacity-40"
              disabled={creating}
              onclick={handleCreate}
            >
              {creating ? "创建中…" : "创建"}
            </button>
          </div>
        {:else}
          <button
            type="button"
            class="inline-flex w-full items-center justify-center gap-1.5 rounded-lg border border-dashed border-tr-border-strong px-3 py-2 text-xs text-tr-accent hover:border-tr-accent-soft-strong hover:bg-tr-accent-soft"
            onclick={() => (showCreate = true)}
          >
            <Plus size={12} strokeWidth={2} aria-hidden="true" />
            新建 SDD 需求
          </button>
        {/if}
      </div>
    </div>
  {/if}
</div>
