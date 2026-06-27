<script lang="ts">
  import type { EnvPlan } from "../types";
  import ModalShell from "./ModalShell.svelte";

  interface Props {
    open: boolean;
    plan: EnvPlan | null;
    onclose: () => void;
  }

  let { open, plan, onclose }: Props = $props();
</script>

<ModalShell
  {open}
  {onclose}
  ariaLabelledby="env-plan-title"
  dialogClass="max-w-[min(92vw,520px)] max-h-[min(85vh,640px)]"
>
  <header class="flex shrink-0 items-start justify-between gap-3 border-b border-white/10 px-5 py-4">
    <div class="min-w-0">
      <h2 id="env-plan-title" class="text-base font-semibold text-white/95">执行计划</h2>
      <p class="mt-0.5 text-xs text-white/45">
        本次集成将按以下顺序执行
        {#if plan}
          <span class="font-medium text-white/70">（{plan.steps.length} 步）</span>
        {/if}
      </p>
    </div>
    <button
      type="button"
      class="shrink-0 rounded-lg border border-white/10 px-2.5 py-1 text-sm text-white/60 hover:bg-white/5"
      onclick={onclose}
      aria-label="关闭"
    >
      ✕
    </button>
  </header>

  <div class="flex-1 overflow-y-auto px-5 py-4">
    {#if plan && plan.steps.length > 0}
      <ol class="space-y-2">
        {#each plan.steps as step, i}
          <li class="flex gap-3 text-sm text-white/70">
            <span class="shrink-0 tabular-nums text-white/35">{i + 1}.</span>
            <span>{step.action}</span>
          </li>
        {/each}
      </ol>
      {#if plan.skipped.length > 0}
        <p class="mt-4 rounded-lg border border-white/8 bg-white/[0.02] px-3 py-2 text-xs text-white/40">
          跳过：{plan.skipped.join("；")}
        </p>
      {/if}
    {:else}
      <p class="text-sm text-white/45">暂无执行步骤。</p>
    {/if}
  </div>
</ModalShell>
