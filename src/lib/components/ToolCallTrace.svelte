<script lang="ts">
  import type { ToolCallRecord } from "../types";
  import { formatDuration, formatTime } from "../timeFormat";

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

  const TOOL_LABELS: Record<string, string> = {
    list_projects: "List projects",
    search_knowledge: "Search knowledge",
    read_doc: "Read document",
    read_agent_pack_meta: "Read pack metadata",
    grep_agent_pack: "Search agent pack",
    read_agent_pack_file: "Read source file",
    list_human_docs: "List human docs",
  };

  function label(name: string): string {
    return TOOL_LABELS[name] ?? name;
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
    if (status === "running") return "Running";
    if (status === "error") return "Failed";
    return "Done";
  }
</script>

{#if toolCalls.length > 0}
  <div class="rounded-lg border border-white/10 bg-black/20">
    <button
      type="button"
      class="flex w-full items-center gap-2 px-3 py-2 text-left text-xs hover:bg-white/[0.03]"
      onclick={() => (panelOpen = !panelOpen)}
    >
      <span class="text-white/35">{panelOpen ? "▾" : "▸"}</span>
      <span class="font-medium text-white/70">
        {toolCalls.length} tool {toolCalls.length === 1 ? "call" : "calls"}
      </span>
      {#if toolCalls.some((c) => c.status === "running")}
        <span class="ml-auto inline-block h-3 w-3 animate-spin rounded-full border-2 border-indigo-300/80 border-t-transparent"></span>
      {/if}
    </button>

    {#if panelOpen}
      <div class="space-y-1 border-t border-white/5 p-2">
        {#each toolCalls as call (call.id)}
          {@const open = expandedIds.has(call.id)}
          <div class="rounded-md border border-white/5 bg-white/[0.02]">
            <button
              type="button"
              class="flex w-full items-start gap-2 px-2.5 py-2 text-left text-xs hover:bg-white/[0.03]"
              onclick={() => toggleItem(call.id)}
            >
              <span class="mt-0.5 text-white/30">{open ? "▾" : "▸"}</span>
              <div class="min-w-0 flex-1">
                <div class="flex flex-wrap items-center gap-2">
                  <span class="font-medium text-indigo-200/90">{label(call.name)}</span>
                  <code class="rounded bg-black/30 px-1 py-0.5 font-mono text-[10px] text-white/45">
                    {call.name}
                  </code>
                  <span
                    class={`rounded px-1.5 py-0.5 text-[10px] ${
                      call.status === "running"
                        ? "bg-amber-500/15 text-amber-200"
                        : call.status === "error"
                          ? "bg-red-500/15 text-red-200"
                          : "bg-emerald-500/15 text-emerald-200"
                    }`}
                  >
                    {statusLabel(call.status)}
                  </span>
                </div>
                {#if summary(call)}
                  <p class="mt-1 truncate font-mono text-[11px] text-white/45">{summary(call)}</p>
                {/if}
                <p class="mt-1 text-[10px] text-white/30">
                  {formatTime(call.started_at)}
                  {#if call.duration_ms != null}
                    · {formatDuration(call.duration_ms)}
                  {:else if call.status === "running"}
                    · running…
                  {/if}
                </p>
              </div>
            </button>

            {#if open}
              <div class="space-y-2 border-t border-white/5 px-3 py-2">
                <div>
                  <p class="mb-1 text-[10px] uppercase tracking-wide text-white/30">Arguments</p>
                  <pre class="max-h-48 overflow-auto rounded bg-black/30 p-2 font-mono text-[11px] leading-relaxed text-white/70">{formatJson(call.arguments)}</pre>
                </div>
                {#if call.error}
                  <div>
                    <p class="mb-1 text-[10px] uppercase tracking-wide text-red-300/70">Error</p>
                    <pre class="overflow-auto rounded bg-red-500/10 p-2 font-mono text-[11px] text-red-200">{call.error}</pre>
                  </div>
                {:else if call.result !== undefined}
                  <div>
                    <p class="mb-1 text-[10px] uppercase tracking-wide text-white/30">Result</p>
                    <pre class="max-h-64 overflow-auto rounded bg-black/30 p-2 font-mono text-[11px] leading-relaxed text-emerald-100/80">{formatJson(call.result)}</pre>
                  </div>
                {:else if call.status === "running"}
                  <p class="text-[11px] text-white/40">Waiting for result…</p>
                {/if}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}
