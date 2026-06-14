<script lang="ts">
  import type { AppTab } from "../types";
  import { TERMS } from "../terminology";

  interface Props {
    active: AppTab;
    disabled?: boolean;
    onchange: (tab: AppTab) => void;
  }

  let { active, disabled = false, onchange }: Props = $props();

  const tabs: { id: AppTab; label: string }[] = [
    { id: "overview", label: "项目概览" },
    { id: "knowledge", label: TERMS.knowledgeTab },
    { id: "env", label: TERMS.agentEnv },
    { id: "sdd", label: "SDD 工作流" },
  ];
</script>

<nav class="flex items-center gap-1 rounded-lg border border-white/10 bg-white/[0.03] p-0.5" aria-label="Main navigation">
  {#each tabs as tab}
    <button
      type="button"
      class={`rounded-md px-3 py-1.5 text-sm transition-colors disabled:opacity-40 ${
        active === tab.id
          ? "bg-indigo-600 font-medium text-white shadow-sm"
          : "text-white/60 hover:bg-white/5 hover:text-white/90"
      }`}
      disabled={disabled}
      aria-current={active === tab.id ? "page" : undefined}
      onclick={() => onchange(tab.id)}
    >
      {tab.label}
    </button>
  {/each}
</nav>
