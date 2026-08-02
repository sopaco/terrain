<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { Check } from "@lucide/svelte";
  import {
    createSddSession,
    deleteSddSession,
    getSddStatus,
    readDocument,
    runSddPhase,
    saveSddOutput,
    setActiveSddSession,
  } from "../api";
  import type { SddPhase, SddPhaseInfo, SddStatus } from "../types";
  import { TERMS } from "../terminology";
  import MarkdownViewer from "./MarkdownViewer.svelte";
  import SddSessionSelector from "./SddSessionSelector.svelte";

  interface Props {
    projectSlug: string | null;
    repoPath: string | null;
    acpOk: boolean;
    llmReady: boolean;
    hybridNativeLlm?: boolean;
    onStatus?: (message: string, kind: "idle" | "loading" | "progress" | "success" | "error") => void;
  }

  let { projectSlug, repoPath, acpOk, llmReady, hybridNativeLlm = false, onStatus }: Props = $props();

  let status = $state<SddStatus | null>(null);
  let loading = $state(false);
  let busyPhase = $state<SddPhase | null>(null);
  let requirementInput = $state("");
  let hitlFeedback = $state("");
  let activeOutput = $state<string | null>(null);
  let activePhase = $state<SddPhase | null>(null);
  let outputBody = $state("");
  let editBody = $state("");
  let outputLoading = $state(false);
  let saving = $state(false);
  let sessionBusy = $state(false);
  let dirty = $state(false);
  let viewMode = $state<"preview" | "edit">("preview");
  let sessionPickerOpen = $state(false);

  const phaseDescriptions: Record<SddPhase, string> = {
    requirements: "交互式澄清需求、用户故事与验收标准，产出结构化需求文档。",
    tech_design: `结合 ${TERMS.humanKnowledge} 与 ${TERMS.agentKnowledge}，输出可实施的技术方案。`,
    code_gen: "委派 OpenCode 按技术方案在仓库中实现代码（需 OpenCode）。",
    code_review: "对照需求与方案，对实现进行智能 Code Review。",
  };

  const activePhaseLabel = $derived(
    status?.phases.find((p) => p.phase === activePhase)?.label ?? "",
  );

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

  function buildUserInput(phase: SddPhase, includeFeedback: boolean): string | undefined {
    const parts: string[] = [];
    if (phase === "requirements" && requirementInput.trim()) {
      parts.push(requirementInput.trim());
    }
    if (includeFeedback && hitlFeedback.trim()) {
      parts.push(hitlFeedback.trim());
    }
    return parts.length > 0 ? parts.join("\n\n") : undefined;
  }

  async function runPhase(phase: SddPhase, includeFeedback = false) {
    if (!projectSlug || !repoPath) {
      onStatus?.("请先选择已索引的项目。", "error");
      return;
    }
    if (!status?.active_session_id) {
      onStatus?.("请先新建或选择一个 SDD 需求。", "error");
      return;
    }
    if (!acpOk && (phase === "code_gen" || !hybridNativeLlm)) {
      onStatus?.(
        phase === "code_gen"
          ? "代码生成需要 ACP 代理，请在设置中配置并确保其在 PATH 上。"
          : "请先在设置中配置 ACP 代理。",
        "error",
      );
      return;
    }
    if (hybridNativeLlm && phase !== "code_gen" && !llmReady) {
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
        buildUserInput(phase, includeFeedback),
        status.active_session_id,
      );
      onStatus?.(`${result.output_path.split("/").pop()} 已生成`, "success");
      hitlFeedback = "";
      await loadStatus();
      await viewOutput(result.output_path, phase, true);
    } catch (e) {
      onStatus?.(String(e), "error");
    } finally {
      busyPhase = null;
    }
  }

  async function viewOutput(path: string, phase?: SddPhase, skipDirtyCheck = false) {
    if (!skipDirtyCheck && dirty && !confirm("有未保存的编辑，确定切换文档吗？")) return;
    activeOutput = path;
    activePhase = phase ?? status?.phases.find((p) => p.output_path === path)?.phase ?? null;
    outputLoading = true;
    viewMode = "preview";
    try {
      const doc = await readDocument(path);
      outputBody = doc.body;
      editBody = doc.body;
      dirty = false;
    } catch (e) {
      outputBody = `_无法读取文档：${e}_`;
      editBody = outputBody;
    } finally {
      outputLoading = false;
    }
  }

  async function saveEdits() {
    if (!activeOutput) return;
    saving = true;
    try {
      await saveSddOutput(activeOutput, editBody);
      outputBody = editBody;
      dirty = false;
      viewMode = "preview";
      onStatus?.("已保存人工修改", "success");
      await loadStatus();
    } catch (e) {
      onStatus?.(String(e), "error");
    } finally {
      saving = false;
    }
  }

  async function submitHitlRevision() {
    if (!activePhase) {
      onStatus?.("请先打开某一阶段的输出文档。", "error");
      return;
    }
    if (!hitlFeedback.trim()) {
      onStatus?.("请填写修订反馈后再提交。", "error");
      return;
    }
    if (dirty) {
      await saveEdits();
    }
    await runPhase(activePhase, true);
  }

  async function switchSession(sessionId: string) {
    if (!projectSlug || sessionId === status?.active_session_id) {
      sessionPickerOpen = false;
      return;
    }
    if (dirty && !confirm("有未保存的编辑，确定切换需求吗？")) return;
    sessionPickerOpen = false;
    try {
      status = await setActiveSddSession(projectSlug, sessionId);
      activeOutput = null;
      outputBody = "";
      editBody = "";
      hitlFeedback = "";
      dirty = false;
      onStatus?.("已切换需求", "success");
    } catch (e) {
      onStatus?.(String(e), "error");
    }
  }

  async function createSession(title: string) {
    if (!projectSlug) return;
    sessionBusy = true;
    try {
      await createSddSession(projectSlug, title);
      sessionPickerOpen = false;
      requirementInput = "";
      activeOutput = null;
      outputBody = "";
      editBody = "";
      dirty = false;
      await loadStatus();
      onStatus?.(`已创建需求：${title}`, "success");
    } catch (e) {
      onStatus?.(String(e), "error");
    } finally {
      sessionBusy = false;
    }
  }

  async function deleteSession(sessionId: string) {
    if (!projectSlug) return;
    sessionBusy = true;
    try {
      status = await deleteSddSession(projectSlug, sessionId);
      if (activeOutput && !status.phases.some((p) => p.output_path === activeOutput)) {
        activeOutput = null;
        outputBody = "";
        editBody = "";
        activePhase = null;
        hitlFeedback = "";
        dirty = false;
      }
      onStatus?.("已删除需求及关联产出", "success");
    } catch (e) {
      onStatus?.(String(e), "error");
    } finally {
      sessionBusy = false;
    }
  }

  function canRun(phaseInfo: SddPhaseInfo): boolean {
    if (busyPhase || !status?.active_session_id) return false;
    const idx = status?.phases.findIndex((p) => p.phase === phaseInfo.phase) ?? -1;
    if (idx <= 0) return true;
    return status?.phases[idx - 1]?.ready ?? false;
  }

  function onEditInput() {
    dirty = editBody !== outputBody;
  }

  $effect(() => {
    if (projectSlug) {
      void loadStatus();
      activeOutput = null;
      outputBody = "";
      editBody = "";
      hitlFeedback = "";
      dirty = false;
      sessionPickerOpen = false;
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
  <div class="w-[420px] shrink-0 overflow-y-auto border-r border-tr-border-strong bg-tr-surface p-5">
    {#if !projectSlug}
      <div class="flex h-full flex-col items-center justify-center text-center text-sm text-tr-ink-3">
        <p>选择项目以开始 SDD 工作流</p>
      </div>
    {:else if loading && !status}
      <div class="flex h-full items-center justify-center text-sm text-tr-ink-3">加载中…</div>
    {:else if status}
      <div class="mb-4">
        <h2 class="text-lg font-semibold">SDD 标准化工作流</h2>
        <p class="mt-1 text-xs text-tr-ink-3">需求澄清 → 技术方案 → 代码生成 → Code Review</p>
        {#if !status.skill_ready}
          <p class="mt-2 rounded-lg border border-tr-watch/30 bg-tr-watch-soft px-3 py-2 text-xs text-tr-watch">
            SDD Skill 未找到，部分 ACP 功能可能受限。
          </p>
        {/if}
      </div>

      <SddSessionSelector
        sessions={status.sessions}
        activeSessionId={status.active_session_id}
        open={sessionPickerOpen}
        creating={sessionBusy}
        ontoggle={() => (sessionPickerOpen = !sessionPickerOpen)}
        onselect={switchSession}
        oncreate={createSession}
        ondelete={deleteSession}
      />

      {#if !status.active_session_id}
        <p class="mb-4 rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-3 text-xs text-tr-ink-3">
          从上方选择器新建 SDD 需求后，即可开始四阶段工作流。
        </p>
      {:else}
        <div class="space-y-4">
          {#each status.phases as phaseInfo, i}
            <div
              class={`rounded-xl border p-4 transition-colors ${
                phaseInfo.ready
                  ? "border-tr-good/35 bg-tr-good-soft"
                  : busyPhase === phaseInfo.phase
                    ? "border-tr-accent-soft-strong bg-tr-accent-soft"
                    : "border-tr-border-strong bg-tr-elevated"
              }`}
            >
              <div class="flex items-start gap-3">
                <div
                  class={`mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-xs font-bold ${
                    phaseInfo.ready
                      ? "bg-tr-good-soft text-tr-good"
                      : "bg-tr-elevated text-tr-ink-3"
                  }`}
                >
                  {#if phaseInfo.ready}
                    <Check size={14} strokeWidth={2.5} aria-hidden="true" />
                  {:else}
                    {i + 1}
                  {/if}
                </div>
                <div class="min-w-0 flex-1">
                  <h3 class="font-medium">{phaseInfo.label}</h3>
                  <p class="mt-1 text-xs leading-relaxed text-tr-ink-3">
                    {phaseDescriptions[phaseInfo.phase]}
                  </p>
                  {#if phaseInfo.updated_at}
                    <p class="mt-2 text-[10px] text-tr-ink-3">{phaseInfo.updated_at}</p>
                  {/if}

                  {#if phaseInfo.phase === "requirements"}
                    <textarea
                      class="mt-3 w-full rounded-lg border border-tr-border-strong bg-tr-page px-3 py-2 text-xs outline-none focus:border-tr-accent"
                      rows="3"
                      placeholder="描述你的需求、背景或用户故事…"
                      bind:value={requirementInput}
                      disabled={!!busyPhase}
                    ></textarea>
                  {/if}

                  <div class="mt-3 flex flex-wrap gap-2">
                    <button
                      type="button"
                      class="tr-press rounded-lg bg-tr-accent px-3 py-1.5 text-xs font-medium transition-colors hover:bg-tr-accent-hover disabled:opacity-40"
                      disabled={!canRun(phaseInfo)}
                      onclick={() => runPhase(phaseInfo.phase)}
                    >
                      {busyPhase === phaseInfo.phase ? "运行中…" : phaseInfo.ready ? "重新生成" : "运行"}
                    </button>
                    {#if phaseInfo.ready}
                      <button
                        type="button"
                        class={`rounded-lg border px-3 py-1.5 text-xs hover:bg-tr-elevated ${
                          activeOutput === phaseInfo.output_path
                            ? "border-tr-accent-soft-strong bg-tr-accent-soft"
                            : "border-tr-border-strong"
                        }`}
                        onclick={() => viewOutput(phaseInfo.output_path, phaseInfo.phase)}
                      >
                        审核输出
                      </button>
                    {/if}
                  </div>
                </div>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    {/if}
  </div>

  <main class="flex min-w-0 flex-1 flex-col overflow-hidden">
    {#if outputLoading}
      <div class="flex flex-1 items-center justify-center text-sm text-tr-ink-3">加载文档…</div>
    {:else if activeOutput}
      <header class="flex shrink-0 items-center gap-2 border-b border-tr-border-strong bg-tr-surface/60 px-5 py-3">
        <div class="min-w-0 flex-1">
          <p class="text-sm font-medium text-tr-ink-2">{activePhaseLabel}</p>
          <p class="truncate font-mono text-[10px] text-tr-ink-3">{activeOutput}</p>
        </div>
        <div class="flex shrink-0 items-center gap-1 rounded-lg border border-tr-border-strong p-0.5">
          <button
            type="button"
            class={`rounded-md px-2.5 py-1 text-xs ${viewMode === "preview" ? "bg-tr-elevated text-white" : "text-tr-ink-3 hover:text-tr-ink-2"}`}
            onclick={() => (viewMode = "preview")}
          >
            预览
          </button>
          <button
            type="button"
            class={`rounded-md px-2.5 py-1 text-xs ${viewMode === "edit" ? "bg-tr-elevated text-white" : "text-tr-ink-3 hover:text-tr-ink-2"}`}
            onclick={() => (viewMode = "edit")}
          >
            编辑
          </button>
        </div>
        {#if viewMode === "edit"}
          <button
            type="button"
            class="tr-press rounded-lg border border-tr-good/35 bg-tr-good-soft px-3 py-1.5 text-xs font-medium text-tr-good transition-colors hover:bg-tr-good-soft/70 disabled:opacity-40"
            disabled={!dirty || saving}
            onclick={saveEdits}
          >
            {saving ? "保存中…" : dirty ? "保存修改" : "已保存"}
          </button>
        {/if}
      </header>

      <div class="min-h-0 flex-1 overflow-y-auto">
        {#if viewMode === "preview"}
          <article class="px-8 py-8">
            <MarkdownViewer body={outputBody} repoPath={repoPath} />
          </article>
        {:else}
          <div class="flex h-full flex-col p-5">
            <textarea
              class="min-h-0 flex-1 resize-none rounded-xl border border-tr-border-strong bg-tr-page p-4 font-mono text-xs leading-relaxed outline-none focus:border-tr-accent"
              bind:value={editBody}
              oninput={onEditInput}
            ></textarea>
          </div>
        {/if}
      </div>

      <footer class="shrink-0 border-t border-tr-accent-soft-strong bg-tr-accent-soft px-5 py-4">
        <div class="mb-2 flex items-center justify-between gap-2">
          <div>
            <p class="text-xs font-medium text-tr-ink-2">人工修订（HITL）</p>
            <p class="text-[10px] text-tr-ink-3">填写反馈后点击按钮，模型将基于当前文档与你的意见重新生成</p>
          </div>
        </div>
        <textarea
          id="hitl-feedback"
          class="mb-3 w-full rounded-lg border border-tr-border-strong bg-tr-page px-3 py-2 text-xs outline-none focus:border-tr-accent"
          rows="3"
          placeholder="例如：补充验收标准、调整技术选型、修正术语、缩小实现范围…"
          bind:value={hitlFeedback}
          disabled={!!busyPhase}
        ></textarea>
        <div class="flex flex-wrap items-center justify-end gap-2">
          {#if dirty}
            <button
              type="button"
              class="tr-press rounded-lg border border-tr-border-strong px-3 py-1.5 text-xs text-tr-ink-2 transition-colors hover:bg-tr-elevated disabled:opacity-40"
              disabled={saving || !!busyPhase}
              onclick={saveEdits}
            >
              先保存编辑
            </button>
          {/if}
          <button
            type="button"
            class="tr-press rounded-lg bg-tr-accent px-4 py-2 text-xs font-medium transition-colors hover:bg-tr-accent-hover disabled:opacity-40"
            disabled={!!busyPhase || !hitlFeedback.trim()}
            onclick={submitHitlRevision}
          >
            {busyPhase ? "修订中…" : "提交反馈并修订"}
          </button>
        </div>
      </footer>
    {:else}
      <div class="flex h-full flex-col items-center justify-center gap-2 px-8 text-center text-tr-ink-3">
        <p class="text-tr-ink-2">阶段输出将显示在这里</p>
        <p class="text-sm">完成任一阶段后点击「审核输出」，在下方提交修订反馈</p>
      </div>
    {/if}
  </main>
</div>
