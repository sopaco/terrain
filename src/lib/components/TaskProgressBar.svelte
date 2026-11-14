<script lang="ts">
  import { Check, Minus } from "@lucide/svelte";
  import { tr } from "../i18n";

  export type ProgressStepState = "done" | "active" | "pending" | "skipped";

  export interface ProgressStep {
    key: string;
    label: string;
    state: ProgressStepState;
    /** Explanation for skipped/pending steps, surfaced as a tooltip. */
    title?: string;
  }

  interface Props {
    /** What is running, e.g. "Importing and initializing". */
    title: string;
    /** The project the task belongs to — never the merely-selected project. */
    projectName: string;
    projectPath?: string | null;
    stage?: string | null;
    message: string;
    steps?: ProgressStep[];
    /** Contextual hint (auto focus / background task / switch target). */
    note?: string | null;
    /** 0–100 when the stage makes it meaningful; null renders an indeterminate bar. */
    percent?: number | null;
    onswitch?: (() => void) | null;
    switchLabel?: string | null;
  }

  let {
    title,
    projectName,
    projectPath = null,
    stage = null,
    message,
    steps = [],
    note = null,
    percent = null,
    onswitch = null,
    switchLabel = null,
  }: Props = $props();

  const stepChip: Record<ProgressStepState, string> = {
    done: "border-tr-watch/35 bg-tr-watch-soft text-tr-watch",
    active: "border-tr-watch bg-tr-watch text-tr-on-watch",
    pending: "border-tr-border-strong text-tr-ink-3",
    skipped: "border-dashed border-tr-border-strong text-tr-ink-3",
  };
</script>

<div class="shrink-0 overflow-hidden border-b border-tr-watch/30 bg-gradient-to-r from-tr-watch-soft to-transparent px-4 py-3">
  <div class="flex items-start gap-3">
    <div
      class="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border border-tr-watch/30 bg-tr-watch-soft"
      aria-hidden="true"
    >
      <span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-tr-watch border-t-transparent"></span>
    </div>

    <div class="min-w-0 flex-1">
      <div class="flex flex-wrap items-center gap-2">
        <span class="text-sm font-medium text-tr-ink">{title}</span>
        <span
          class="max-w-[16rem] truncate rounded-full bg-tr-page px-2 py-0.5 text-[11px] font-medium text-tr-ink-2"
          title={projectName}
        >
          {projectName}
        </span>
        {#if stage}
          <span class="rounded-full border border-tr-watch/30 px-2 py-0.5 text-[10px] text-tr-watch">
            {stage}
          </span>
        {/if}
      </div>

      {#if projectPath}
        <p
          class="mt-1 truncate font-mono text-[10px] text-tr-ink-3"
          title={projectPath}
        >
          {projectPath}
        </p>
      {/if}

      <p class="mt-1 break-words text-xs leading-relaxed text-tr-watch">
        {message}
      </p>

      {#if steps.length > 0}
        <ol class="mt-2 flex flex-wrap items-center gap-1.5">
          {#each steps as step (step.key)}
            <li
              class={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[10px] ${stepChip[step.state]}`}
              aria-current={step.state === "active" ? "step" : undefined}
              title={step.title}
            >
              {#if step.state === "done"}
                <Check size={10} strokeWidth={3} aria-hidden="true" />
              {:else if step.state === "skipped"}
                <Minus size={10} strokeWidth={3} aria-hidden="true" />
              {/if}
              {step.label}
            </li>
          {/each}
        </ol>
      {/if}

      <div
        class="mt-2 h-1.5 overflow-hidden rounded-full bg-tr-elevated"
        role="progressbar"
        aria-label={title}
        aria-valuemin="0"
        aria-valuemax="100"
        aria-valuenow={percent ?? undefined}
      >
        {#if percent == null}
          <div class="h-full w-[43%] animate-pulse rounded-full bg-tr-watch"></div>
        {:else}
          <div
            class="h-full rounded-full bg-tr-watch transition-[width] duration-500 ease-out"
            style={`width: ${Math.min(100, Math.max(4, percent))}%`}
          ></div>
        {/if}
      </div>

      {#if note || (onswitch && switchLabel)}
        <div class="mt-1.5 flex flex-wrap items-center gap-2">
          {#if note}
            <p class="text-[11px] leading-relaxed text-tr-ink-3">{note}</p>
          {/if}
          {#if onswitch && switchLabel}
            <button
              type="button"
              class="tr-press rounded-md border border-tr-watch/40 px-2 py-0.5 text-[10px] font-medium text-tr-watch transition-colors hover:bg-tr-watch-soft"
              onclick={onswitch}
            >
              {switchLabel}
            </button>
          {/if}
        </div>
      {/if}
    </div>
  </div>
</div>
