<script lang="ts">
  import { BookOpen, LayoutDashboard, SquareTerminal, Workflow } from "@lucide/svelte";
  import type { AppTab } from "../types";
  import { TERMS } from "../terminology";

  interface Props {
    active: AppTab;
    disabled?: boolean;
    onchange: (tab: AppTab) => void;
  }

  let { active, disabled = false, onchange }: Props = $props();

  const tabs: { id: AppTab; label: string; title: string }[] = [
    { id: "overview", label: "概览", title: "项目概览" },
    { id: "knowledge", label: "知识库", title: TERMS.knowledgeTab },
    { id: "env", label: "工程环境", title: TERMS.agentEnv },
    { id: "sdd", label: "SDD", title: "SDD 工作流：规格驱动开发" },
  ];
</script>

{#snippet navButton(tab: (typeof tabs)[number])}
  <button
    type="button"
    class={`tr-press flex w-14 flex-col items-center gap-1 rounded-lg px-1 py-2 text-[10.5px] leading-tight transition-colors disabled:opacity-40 ${
      active === tab.id
        ? "bg-tr-accent-soft text-tr-accent"
        : "text-tr-ink-3 hover:bg-tr-elevated hover:text-tr-ink-2"
    }`}
    disabled={disabled}
    aria-current={active === tab.id ? "page" : undefined}
    title={tab.title}
    onclick={() => onchange(tab.id)}
  >
    {#if tab.id === "overview"}
      <LayoutDashboard size={18} strokeWidth={1.8} aria-hidden="true" />
    {:else if tab.id === "knowledge"}
      <BookOpen size={18} strokeWidth={1.8} aria-hidden="true" />
    {:else if tab.id === "env"}
      <SquareTerminal size={18} strokeWidth={1.8} aria-hidden="true" />
    {:else}
      <Workflow size={18} strokeWidth={1.8} aria-hidden="true" />
    {/if}
    <span class={active === tab.id ? "font-medium text-tr-ink" : ""}>{tab.label}</span>
  </button>
{/snippet}

<nav class="flex flex-col items-stretch gap-1" aria-label="Main navigation">
  {#each tabs as tab (tab.id)}
    {@render navButton(tab)}
  {/each}
</nav>
