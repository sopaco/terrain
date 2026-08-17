<script lang="ts">
  import { Plus, Trash2 } from "@lucide/svelte";
  import type { SddSessionInfo } from "../types";
  import { tr } from "../i18n";
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
    const title = newTitle.trim() || tr("sdd.session.defaultTitle");
    oncreate(title);
    newTitle = "";
    showCreate = false;
  }

  function confirmDelete(session: SddSessionInfo, e: MouseEvent) {
    e.stopPropagation();
    if (!confirm(tr("sdd.session.confirmDelete", { title: session.title }))) {
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
    class="tr-press flex w-full items-center gap-2 rounded-xl border border-tr-border-strong bg-tr-elevated px-3 py-2.5 text-left text-sm transition-colors hover:bg-tr-raised"
    onclick={ontoggle}
    aria-expanded={open}
    aria-haspopup="listbox"
  >
    <span class="min-w-0 flex-1">
      <span class="block text-[10px] text-tr-ink-3">{tr("sdd.session.current")}</span>
      <span class="block truncate font-medium text-tr-ink">
        {active?.title ?? tr("sdd.session.placeholder")}
      </span>
    </span>
    <ChevronIcon direction={open ? "up" : "down"} size={14} />
  </button>

  {#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="fixed inset-0 z-[200]" onclick={ontoggle} role="presentation"></div>
    <div
      class="tr-pop fixed z-[201] overflow-hidden rounded-xl border border-tr-border-strong bg-tr-raised shadow-xl"
      style="top: {menuTop}px; left: {menuLeft}px; width: {menuWidth}px; --tr-pop-origin: top left;"
      role="listbox"
    >
      <div class="border-b border-tr-border-strong px-3 py-2 text-xs text-tr-ink-3">
        {tr("sdd.session.count", { count: sessions.length })}
      </div>
      <ul class="max-h-56 overflow-y-auto py-1">
        {#each sessions as session}
          <li class="mx-1 flex items-stretch gap-0.5">
            <button
              type="button"
              class={`flex min-w-0 flex-1 flex-col rounded-lg px-3 py-2 text-left text-sm transition-colors hover:bg-tr-elevated ${
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
              class="tr-press inline-flex shrink-0 items-center justify-center rounded-lg px-2 text-tr-ink-3 transition-colors hover:bg-tr-critical-soft hover:text-tr-critical"
              title={tr("sdd.session.deleteTitle")}
              aria-label={tr("sdd.session.deleteAria", { title: session.title })}
              onclick={(e) => confirmDelete(session, e)}
            >
              <Trash2 size={14} strokeWidth={2} aria-hidden="true" />
            </button>
          </li>
        {:else}
          <li class="px-3 py-3 text-xs text-tr-ink-3">{tr("sdd.session.empty")}</li>
        {/each}
      </ul>
      <div class="border-t border-tr-border-strong p-2">
        {#if showCreate}
          <div class="flex gap-2">
            <input
              class="min-w-0 flex-1 rounded-lg border border-tr-border-strong bg-tr-page px-2.5 py-1.5 text-xs outline-none focus:border-tr-accent"
              placeholder={tr("sdd.session.titlePlaceholder")}
              bind:value={newTitle}
              disabled={creating}
              onkeydown={(e) => e.key === "Enter" && handleCreate()}
            />
            <button
              type="button"
              class="tr-press shrink-0 rounded-lg bg-tr-accent px-3 py-1.5 text-xs font-medium transition-colors hover:bg-tr-accent-hover disabled:opacity-40"
              disabled={creating}
              onclick={handleCreate}
            >
              {creating ? tr("sdd.session.creating") : tr("common.create")}
            </button>
          </div>
        {:else}
          <button
            type="button"
            class="tr-press inline-flex w-full items-center justify-center gap-1.5 rounded-lg border border-dashed border-tr-border-strong px-3 py-2 text-xs text-tr-accent transition-colors hover:border-tr-accent-soft-strong hover:bg-tr-accent-soft"
            onclick={() => (showCreate = true)}
          >
            <Plus size={12} strokeWidth={2} aria-hidden="true" />
            {tr("sdd.session.createNew")}
          </button>
        {/if}
      </div>
    </div>
  {/if}
</div>
