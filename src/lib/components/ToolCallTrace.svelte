<script lang="ts">
  import type { ToolCallRecord } from "../types";
  import { formatDuration, formatTime } from "../timeFormat";
  import ChevronIcon from "./icons/ChevronIcon.svelte";

  import { tr } from "../i18n";

  interface Props {
    toolCalls: ToolCallRecord[];
    defaultExpanded?: boolean;
  }

  let { toolCalls, defaultExpanded = false }: Props = $props();

  let panelOpen = $state(false);
  let expandedIds = $state<Set<string>>(new Set());

  $effect(() => {
    if (defaultExpanded) panelOpen = true;
  });

  function label(name: string): string {
    const key = `terms.tool.${name}`;
    const value = tr(key);
    return value === key ? name : value;
  }

  function summary(call: ToolCallRecord): string {
    const args = call.arguments ?? {};
    switch (call.name) {
      case "search_knowledge":
        return String(args.query ?? "");
      case "grep_agent_pack":
        return `/${args.pattern ?? ""}/`;
      case "read_agent_pack_file":
        return String(args.file_path ?? "");
      case "read_doc":
        return String(args.path ?? "").split("/").pop() ?? String(args.path ?? "");
      case "read_agent_pack_meta":
      case "list_human_docs":
        return String(args.project ?? "");
      default:
        return "";
    }
  }

  function formatJson(value: unknown): string {
    try {
      return JSON.stringify(value, null, 2);
    } catch {
      return String(value);
    }
  }

  function toggleItem(id: string) {
    const next = new Set(expandedIds);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    expandedIds = next;
  }

  function statusLabel(status: ToolCallRecord["status"]): string {
    if (status === "running") return tr("terms.toolStatus.running");
    if (status === "error") return tr("terms.toolStatus.error");
    return tr("terms.toolStatus.ok");
  }
</script>

{#if toolCalls.length > 0}
  <div class="rounded-lg border border-tr-border-strong bg-tr-page">
    <button
      type="button"
      class="flex w-full items-center gap-2 px-3 py-2 text-left text-xs transition-colors hover:bg-tr-elevated"
      onclick={() => (panelOpen = !panelOpen)}
    >
      <ChevronIcon direction={panelOpen ? "down" : "right"} size={12} class="shrink-0 text-tr-ink-3" />
      <span class="font-medium text-tr-ink-2">
        {toolCalls.length === 1
          ? tr("ask.toolTrace.headerOne")
          : tr("ask.toolTrace.headerMany", { count: toolCalls.length })}
      </span>
      {#if toolCalls.some((c) => c.status === "running")}
        <span class="ml-auto inline-block h-3 w-3 animate-spin rounded-full border-2 border-tr-accent border-t-transparent"></span>
      {/if}
    </button>

    {#if panelOpen}
      <div class="space-y-1 border-t border-tr-border p-2">
        {#each toolCalls as call (call.id)}
          {@const open = expandedIds.has(call.id)}
          <div class="rounded-md border border-tr-border bg-tr-elevated">
            <button
              type="button"
              class="flex w-full items-start gap-2 px-2.5 py-2 text-left text-xs transition-colors hover:bg-tr-elevated"
              onclick={() => toggleItem(call.id)}
            >
              <span class="mt-0.5 inline-flex shrink-0 text-tr-ink-3">
                <ChevronIcon direction={open ? "down" : "right"} size={12} />
              </span>
              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-2">
                  <span class="font-medium text-tr-accent">{label(call.name)}</span>
                  <code class="rounded bg-tr-page px-1 py-0.5 font-mono text-[10px] text-tr-ink-3">
                    {call.name}
                  </code>
                  <span
                    class={`rounded px-1.5 py-0.5 text-[10px] ${
                      call.status === "running"
                        ? "bg-tr-watch-soft text-tr-watch"
                        : call.status === "error"
                          ? "bg-tr-critical-soft text-tr-critical"
                          : "bg-tr-good-soft text-tr-good"
                    }`}
                  >
                    {statusLabel(call.status)}
                  </span>
                </div>
                {#if summary(call)}
                  <p class="mt-1 truncate font-mono text-[11px] text-tr-ink-3">{summary(call)}</p>
                {/if}
                <p class="mt-1 text-[10px] text-tr-ink-3">
                  {formatTime(call.started_at)}
                  {#if call.duration_ms != null}
                    · {formatDuration(call.duration_ms)}
                  {:else if call.status === "running"}
                    · {tr("terms.toolStatus.running")}…
                  {/if}
                </p>
              </div>
            </button>

            {#if open}
              <div class="space-y-2 border-t border-tr-border px-3 py-2">
                <div>
                  <p class="mb-1 text-[10px] uppercase tracking-wide text-tr-ink-3">{tr("ask.toolTrace.arguments")}</p>
                  <pre class="max-h-48 overflow-auto rounded bg-tr-page p-2 font-mono text-[11px] leading-relaxed text-tr-ink-2">{formatJson(call.arguments)}</pre>
                </div>
                {#if call.error}
                  <div>
                    <p class="mb-1 text-[10px] uppercase tracking-wide text-tr-critical">{tr("common.error")}</p>
                    <pre class="overflow-auto rounded bg-tr-critical-soft p-2 font-mono text-[11px] text-tr-critical">{call.error}</pre>
                  </div>
                {:else if call.result !== undefined}
                  <div>
                    <p class="mb-1 text-[10px] uppercase tracking-wide text-tr-ink-3">{tr("ask.toolTrace.result")}</p>
                    <pre class="max-h-64 overflow-auto rounded bg-tr-page p-2 font-mono text-[11px] leading-relaxed text-tr-good">{formatJson(call.result)}</pre>
                  </div>
                {:else if call.status === "running"}
                  <p class="text-[11px] text-tr-ink-3">{tr("ask.toolTrace.waiting")}</p>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}
