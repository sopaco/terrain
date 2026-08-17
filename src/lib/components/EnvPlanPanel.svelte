<script lang="ts">
  import type { EnvPlan } from "../types";
  import { tr } from "../i18n";
  import CloseButton from "./icons/CloseButton.svelte";
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
  <header class="flex shrink-0 items-start justify-between gap-3 border-b border-tr-border-strong px-5 py-4">
    <div class="min-w-0">
      <h2 id="env-plan-title" class="text-base font-semibold text-tr-ink">{tr("env.plan.title")}</h2>
      <p class="mt-0.5 text-xs text-tr-ink-3">
        {tr("env.plan.subtitle")}
        {#if plan}
          <span class="font-medium text-tr-ink-2">{tr("env.plan.stepCount", { count: plan.steps.length })}</span>
        {/if}
      </p>
    </div>
    <CloseButton onclick={onclose} class="py-1 text-sm" />
  </header>

  <div class="flex-1 overflow-y-auto px-5 py-4">
    {#if plan && plan.steps.length > 0}
      <ol class="space-y-2">
        {#each plan.steps as step, i}
          <li class="flex gap-3 text-sm text-tr-ink-2">
            <span class="shrink-0 tabular-nums text-tr-ink-3">{i + 1}.</span>
            <span>{step.action}</span>
          </li>
        {/each}
      </ol>
      {#if plan.skipped.length > 0}
        <p class="mt-4 rounded-lg border border-tr-border bg-tr-elevated px-3 py-2 text-xs text-tr-ink-3">
          {tr("env.plan.skipped", { items: plan.skipped.join(tr("env.plan.skipSeparator")) })}
        </p>
      {/if}
    {:else}
      <p class="text-sm text-tr-ink-3">{tr("env.plan.empty")}</p>
    {/if}
  </div>
</ModalShell>
