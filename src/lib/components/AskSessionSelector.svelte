<script lang="ts">
  import { History, Plus, Trash2 } from "@lucide/svelte";
  import { tr } from "../i18n";
  import type { AskSessionInfo } from "../types";

  interface Props {
    sessions: AskSessionInfo[];
    activeSessionId?: string | null;
    open: boolean;
    creating?: boolean;
    ontoggle: () => void;
    onselect: (sessionId: string) => void;
    oncreate: () => void;
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

  let triggerEl = $state<HTMLButtonElement | null>(null);
  let menuTop = $state(0);
  let menuRight = $state(0);
  const menuWidth = 320;

  function updateMenuPosition() {
    if (!triggerEl) return;
    const rect = triggerEl.getBoundingClientRect();
    menuTop = rect.bottom + 6;
    menuRight = Math.max(8, window.innerWidth - rect.right);
  }

  function confirmDelete(session: AskSessionInfo, e: MouseEvent) {
    e.stopPropagation();
    if (!confirm(tr("ask.session.confirmDelete", { title: session.title }))) {
      return;
    }
    ondelete(session.id);
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

<div class="relative shrink-0">
  <button
    type="button"
    bind:this={triggerEl}
    class={`tr-press inline-flex h-9 w-9 items-center justify-center rounded-lg border border-tr-border-strong text-tr-ink-2 transition-colors hover:bg-tr-elevated disabled:cursor-not-allowed disabled:opacity-40 ${
      open ? "border-tr-accent-soft-strong bg-tr-accent-soft text-tr-accent" : ""
    }`}
    onclick={ontoggle}
    aria-expanded={open}
    aria-haspopup="listbox"
    aria-label={tr("ask.session.history")}
    title={tr("ask.session.history")}
    disabled={creating}
  >
    <History size={16} strokeWidth={2} aria-hidden="true" />
  </button>

  {#if open}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="fixed inset-0 z-[200]" onclick={ontoggle} role="presentation"></div>
    <div
      class="tr-pop fixed z-[201] overflow-hidden rounded-xl border border-tr-border-strong bg-tr-surface shadow-2xl"
      style="top: {menuTop}px; right: {menuRight}px; width: {menuWidth}px; --tr-pop-origin: top right;"
      role="listbox"
      aria-label={tr("ask.session.history")}
    >
      <div class="border-b border-tr-border-strong px-3.5 py-2.5">
        <p class="text-sm font-medium text-tr-ink">{tr("ask.session.historyTitle")}</p>
        <p class="text-[11px] text-tr-ink-3">
          {tr("ask.session.recentCount", { count: sessions.length })}
        </p>
      </div>
      <ul class="max-h-64 overflow-y-auto py-1.5">
        {#each sessions as session (session.id)}
          <li class="mx-1.5 flex items-stretch gap-0.5">
            <button
              type="button"
              class={`flex min-w-0 flex-1 flex-col rounded-lg px-3 py-2 text-left transition-colors hover:bg-tr-elevated ${
                activeSessionId === session.id ? "bg-tr-accent-soft-strong" : ""
              }`}
              onclick={() => onselect(session.id)}
              role="option"
              aria-selected={activeSessionId === session.id}
            >
              <span
                class={`truncate text-sm font-medium ${
                  activeSessionId === session.id ? "text-tr-accent" : "text-tr-ink"
                }`}
              >
                {session.title}
              </span>
              {#if session.last_replied_at}
                <span class="text-[10px] text-tr-ink-3">{session.last_replied_at}</span>
              {/if}
            </button>
            <button
              type="button"
              class="tr-press inline-flex shrink-0 items-center justify-center rounded-lg px-2 text-tr-ink-3 transition-colors hover:bg-tr-critical-soft hover:text-tr-critical"
              title={tr("ask.session.deleteThis")}
              aria-label={tr("ask.session.deleteNamed", { title: session.title })}
              onclick={(e) => confirmDelete(session, e)}
            >
              <Trash2 size={14} strokeWidth={2} aria-hidden="true" />
            </button>
          </li>
        {:else}
          <li class="px-3.5 py-4 text-center text-xs text-tr-ink-3">{tr("ask.session.empty")}</li>
        {/each}
      </ul>
      <div class="border-t border-tr-border-strong p-2">
        <button
          type="button"
          class="tr-press inline-flex w-full items-center justify-center gap-1.5 rounded-lg border border-dashed border-tr-border-strong px-3 py-2 text-xs text-tr-accent transition-colors hover:border-tr-accent-soft-strong hover:bg-tr-accent-soft disabled:opacity-40"
          disabled={creating}
          onclick={oncreate}
        >
          <Plus size={12} strokeWidth={2} aria-hidden="true" />
          {creating ? tr("ask.session.creating") : tr("ask.session.newSession")}
        </button>
      </div>
    </div>
  {/if}
</div>
