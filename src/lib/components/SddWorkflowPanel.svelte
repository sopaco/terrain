<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { getSddStatus, readDocument, runSddPhase } from "../api";
  import type { SddPhase, SddPhaseInfo, SddStatus } from "../types";
  import MarkdownViewer from "./MarkdownViewer.svelte";

  interface Props {
    projectSlug: string | null;
    repoPath: string | null;
    acpOk: boolean;
    llmReady: boolean;
    onStatus?: (message: string, kind: "idle" | "loading" | "progress" | "success" | "error") => void;
  }

  let { projectSlug, repoPath, acpOk, llmReady, onStatus }: Props = $props();

  let status = $state<SddStatus | null>(null);
  let loading = $state(false);
  let busyPhase = $state<SddPhase | null>(null);
  let requirementInput = $state("");
  let activeOutput = $state<string | null>(null);
  let outputBody = $state("");
  let outputLoading = $state(false);

  const phaseDescriptions: Record<SddPhase, string> = {
    requirements: "交互式澄清需求、用户故事与验收标准，产出结构化需求文档。",
    tech_design: "结合 Human 文档与 Agent 上下文，输出可实施的技术方案。",
    code_gen: "委派 OpenCode 按技术方案在仓库中实现代码（需 OpenCode）。",
    code_review: "对照需求与方案，对实现进行智能 Code Review。",
  };

  async function loadStatus() {
    if (!projectSlug) {
      status = null;
      return;
    }
    loading = true;
    try {
      status = await getSddStatus(projectSlug);
    } catch {
      status = null;
    } finally {
      loading = false;
    }
  }

  async function runPhase(phase: SddPhase) {
    if (!projectSlug || !repoPath) {
      onStatus?.("请先选择已索引的项目。", "error");
      return;
    }
    if (phase === "code_gen" && !acpOk) {
      onStatus?.("代码生成需要 ACP 代理，请在设置中配置并确保其在 PATH 上。", "error");
      return;
    }
    if (phase !== "code_gen" && !llmReady) {
      onStatus?.("请先在设置中配置 LLM。", "error");
      return;
    }

    busyPhase = phase;
    onStatus?.(`正在运行：${status?.phases.find((p) => p.phase === phase)?.label ?? phase}`, "progress");

    try {
      const result = await runSddPhase(
        repoPath,
        phase,
        projectSlug,
        phase === "requirements" ? requirementInput : undefined,
      );
      onStatus?.(`${result.output_path.split("/").pop()} 已生成`, "success");
      await loadStatus();
      await viewOutput(result.output_path);
    } catch (e) {
      onStatus?.(String(e), "error");
    } finally {
      busyPhase = null;
    }
  }

  async function viewOutput(path: string) {
    activeOutput = path;
    outputLoading = true;
    try {
      const doc = await readDocument(path);
      outputBody = doc.body;
    } catch (e) {
      outputBody = `_无法读取文档：${e}_`;
    } finally {
      outputLoading = false;
    }
  }

  function canRun(phaseInfo: SddPhaseInfo): boolean {
    if (busyPhase) return false;
    const idx = status?.phases.findIndex((p) => p.phase === phaseInfo.phase) ?? -1;
    if (idx <= 0) return true;
    return status?.phases[idx - 1]?.ready ?? false;
  }

  $effect(() => {
    if (projectSlug) {
      void loadStatus();
      activeOutput = null;
      outputBody = "";
    }
  });

  onMount(() => {
    let unlistenProgress: (() => void) | undefined;
    let unlistenDone: (() => void) | undefined;

    void (async () => {
      unlistenProgress = await listen<{ project_slug: string; message: string }>(
        "sdd-progress",
        (ev) => {
          if (ev.payload.project_slug !== projectSlug) return;
          onStatus?.(ev.payload.message, "progress");
        },
      );
      unlistenDone = await listen<{ project_slug: string }>("sdd-done", async (ev) => {
        if (ev.payload.project_slug !== projectSlug) return;
        await loadStatus();
      });
    })();

    return () => {
      unlistenProgress?.();
      unlistenDone?.();
    };
  });
</script>

<div class="flex h-full min-h-0">
  <div class="w-[420px] shrink-0 overflow-y-auto border-r border-white/10 bg-[#14171c] p-5">
    {#if !projectSlug}
      <div class="flex h-full flex-col items-center justify-center text-center text-sm text-white/40">
        <p>选择项目以开始 SDD 工作流</p>
      </div>
    {:else if loading && !status}
      <div class="flex h-full items-center justify-center text-sm text-white/40">加载中…</div>
    {:else if status}
      <div class="mb-5">
        <h2 class="text-lg font-semibold">SDD 标准化工作流</h2>
        <p class="mt-1 text-xs text-white/40">
          需求澄清 → 技术方案 → 代码生成 → Code Review
        </p>
        {#if !status.skill_ready}
          <p class="mt-2 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-100">
            SDD Skill 未找到，部分 ACP 功能可能受限。
          </p>
        {/if}
      </div>

      <div class="space-y-4">
        {#each status.phases as phaseInfo, i}
          <div
            class={`rounded-xl border p-4 transition-colors ${
              phaseInfo.ready
                ? "border-emerald-500/25 bg-emerald-500/5"
                : busyPhase === phaseInfo.phase
                  ? "border-indigo-500/40 bg-indigo-500/10"
                  : "border-white/10 bg-white/[0.02]"
            }`}
          >
            <div class="flex items-start gap-3">
              <div
                class={`mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-xs font-bold ${
                  phaseInfo.ready
                    ? "bg-emerald-500/20 text-emerald-200"
                    : "bg-white/10 text-white/50"
                }`}
              >
                {phaseInfo.ready ? "✓" : i + 1}
              </div>
              <div class="min-w-0 flex-1">
                <h3 class="font-medium">{phaseInfo.label}</h3>
                <p class="mt-1 text-xs leading-relaxed text-white/45">
                  {phaseDescriptions[phaseInfo.phase]}
                </p>
                {#if phaseInfo.updated_at}
                  <p class="mt-2 text-[10px] text-white/30">{phaseInfo.updated_at}</p>
                {/if}

                {#if phaseInfo.phase === "requirements"}
                  <textarea
                    class="mt-3 w-full rounded-lg border border-white/10 bg-black/20 px-3 py-2 text-xs outline-none focus:border-indigo-500"
                    rows="3"
                    placeholder="描述你的需求、背景或用户故事…"
                    bind:value={requirementInput}
                    disabled={!!busyPhase}
                  ></textarea>
                {/if}

                <div class="mt-3 flex flex-wrap gap-2">
                  <button
                    type="button"
                    class="rounded-lg bg-indigo-600 px-3 py-1.5 text-xs font-medium hover:bg-indigo-500 disabled:opacity-40"
                    disabled={!canRun(phaseInfo)}
                    onclick={() => runPhase(phaseInfo.phase)}
                  >
                    {busyPhase === phaseInfo.phase ? "运行中…" : phaseInfo.ready ? "重新运行" : "运行"}
                  </button>
                  {#if phaseInfo.ready}
                    <button
                      type="button"
                      class="rounded-lg border border-white/10 px-3 py-1.5 text-xs hover:bg-white/5"
                      onclick={() => viewOutput(phaseInfo.output_path)}
                    >
                      查看输出
                    </button>
                  {/if}
                </div>
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <main class="min-w-0 flex-1 overflow-y-auto">
    {#if outputLoading}
      <div class="flex h-full items-center justify-center text-sm text-white/40">加载文档…</div>
    {:else if activeOutput && outputBody}
      <article class="px-8 py-8">
        <p class="mb-4 font-mono text-xs text-white/35">{activeOutput}</p>
        <MarkdownViewer body={outputBody} repoPath={repoPath} />
      </article>
    {:else}
      <div class="flex h-full flex-col items-center justify-center gap-2 px-8 text-center text-white/40">
        <p class="text-white/60">阶段输出将显示在这里</p>
        <p class="text-sm">完成任一阶段后点击「查看输出」</p>
      </div>
    {/if}
  </main>
</div>
