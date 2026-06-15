<script lang="ts">
  import type { ProjectOverview, StaleProjectSummary } from "../types";
  import { generateLabel, TERMS } from "../terminology";
  import FreshnessHelpPanel from "./FreshnessHelpPanel.svelte";

  interface Props {
    overview: ProjectOverview | null;
    loading: boolean;
    acpOk: boolean;
    llmReady: boolean;
    agentContextBusy?: boolean;
    lithoBusy?: boolean;
    repackBusy?: boolean;
    initBusy?: boolean;
    initProgress?: string | null;
    staleProjects?: StaleProjectSummary[];
    onOpenKnowledge: () => void;
    onOpenSdd: () => void;
    onOpenEnv?: () => void;
    onOpenAsk: () => void;
    onGenerateHuman?: () => void;
    onGenerateAgentContext?: () => void;
    onRepack?: () => void;
    onInitializeProject?: (repoPath: string, slug?: string) => void;
    onOpenPath?: (path: string) => void;
    onOpenArchitectureDoc?: () => void;
    onOpenHumanOverview?: () => void;
    onOpenStructured?: () => void;
    quickRefreshBusy?: boolean;
    onQuickRefresh?: () => void;
  }

  let {
    overview,
    loading,
    acpOk,
    llmReady,
    agentContextBusy = false,
    lithoBusy = false,
    repackBusy = false,
    initBusy = false,
    initProgress = null,
    staleProjects = [],
    onOpenKnowledge,
    onOpenSdd,
    onOpenEnv,
    onOpenAsk,
    onGenerateHuman,
    onGenerateAgentContext,
    onRepack,
    onInitializeProject,
    onOpenPath,
    onOpenArchitectureDoc,
    onOpenHumanOverview,
    onOpenStructured,
    quickRefreshBusy = false,
    onQuickRefresh,
  }: Props = $props();

  let freshnessHelpOpen = $state(false);

  const readyCount = $derived(
    overview?.asset_health.filter((a) => a.ready).length ?? 0,
  );

  const needsAssetInit = $derived(
    overview != null && readyCount < overview.asset_health.length,
  );

  const needsEnvSetup = $derived(overview != null && !overview.agent_env.ready);

  const initHint = $derived.by(() => {
    if (llmReady && acpOk) return null;
    const parts: string[] = [];
    if (!llmReady) parts.push(`LLM（${TERMS.agentKnowledge}）`);
    if (!acpOk) parts.push(`ACP（${TERMS.humanKnowledge}）`);
    return `部分步骤需要配置：${parts.join("、")}，可在设置中完成后再试`;
  });

  const knowledgePath = $derived(
    overview?.repo_path ? `${overview.repo_path}/.mind-mesh` : null,
  );

  const freshness = $derived(overview?.freshness ?? null);

  function freshnessTone(score: number): string {
    if (score >= 80) return "text-emerald-200";
    if (score >= 50) return "text-amber-200";
    return "text-rose-200";
  }

  function freshnessBadgeClass(score: number, stale?: boolean): string {
    if (!stale && score >= 80) {
      return "bg-emerald-500/15 text-emerald-200";
    }
    if (score >= 50) {
      return "bg-amber-500/15 text-amber-200";
    }
    return "bg-rose-500/15 text-rose-200";
  }

  function formatSyncedAt(value?: string): string {
    if (!value) return "—";
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return value;
    return date.toLocaleString(undefined, {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function openPath(path: string, event: MouseEvent) {
    event.stopPropagation();
    onOpenPath?.(path);
  }

  function startInit(repoPath: string, slug?: string) {
    onInitializeProject?.(repoPath, slug);
  }
</script>

{#snippet initButton(repoPath: string, slug?: string, label = "添加并初始化")}
  <button
    type="button"
    class="rounded-xl bg-indigo-600 px-5 py-2.5 text-sm font-medium hover:bg-indigo-500 disabled:opacity-50"
    disabled={initBusy || !onInitializeProject}
    onclick={() => startInit(repoPath, slug)}
  >
    {initBusy ? (initProgress ?? "初始化中…") : label}
  </button>
{/snippet}

{#snippet initBanner(repoPath: string, title: string, detail: string, slug?: string)}
  <div class="rounded-2xl border border-amber-500/20 bg-amber-500/[0.06] px-5 py-4">
    <div class="flex flex-wrap items-start justify-between gap-4">
      <div class="min-w-0">
        <p class="text-sm font-semibold text-amber-100/95">{title}</p>
        <p class="mt-1 text-xs leading-relaxed text-white/50">{detail}</p>
        {#if initHint}
          <p class="mt-2 text-[11px] text-amber-200/70">{initHint}</p>
        {/if}
        {#if initBusy && initProgress}
          <p class="mt-2 text-xs text-indigo-200/80">{initProgress}</p>
        {/if}
      </div>
      {@render initButton(repoPath, slug)}
    </div>
  </div>
{/snippet}

{#snippet pathField(label: string, path: string | null | undefined)}
  {#if path}
    <button
      type="button"
      class="group flex w-full flex-col gap-1 rounded-xl border border-white/10 bg-black/20 px-3 py-2.5 text-left transition-colors hover:border-indigo-500/30 hover:bg-indigo-500/[0.06]"
      onclick={(e) => openPath(path, e)}
      title={`在 Finder 中打开：${path}`}
    >
      <span class="text-[10px] font-semibold uppercase tracking-wider text-white/35">{label}</span>
      <span class="truncate font-mono text-xs text-indigo-200/90 group-hover:text-indigo-100">
        {path}
      </span>
    </button>
  {/if}
{/snippet}

{#snippet navCard(params: {
  title: string;
  subtitle: string;
  ready: boolean;
  meta: string;
  icon: string;
  onClick?: () => void;
  onGenerate?: () => void;
  generateLabel?: string;
  regenerateLabel?: string;
  generateDisabled?: boolean;
})}
  {#if params.ready && params.onClick}
    <div class="flex flex-col overflow-hidden rounded-2xl border border-white/10 bg-white/[0.02]">
      <button
        type="button"
        class="group flex w-full flex-col p-5 text-left transition-colors hover:bg-indigo-500/[0.05]"
        onclick={params.onClick}
      >
        <div class="flex items-start justify-between gap-3">
          <div class="flex min-w-0 items-start gap-3">
            <span
              class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-indigo-500/15 text-lg"
              aria-hidden="true"
            >{params.icon}</span>
            <div class="min-w-0">
              <p class="text-sm font-semibold text-white/90">{params.title}</p>
              <p class="mt-0.5 text-xs text-white/45">{params.subtitle}</p>
            </div>
          </div>
          <span class="shrink-0 text-white/25 transition-transform group-hover:translate-x-0.5 group-hover:text-indigo-300">
            →
          </span>
        </div>
        <div class="mt-4 flex items-center justify-between gap-2">
          <span class="text-sm text-white/55">{params.meta}</span>
          <span class="rounded-full bg-emerald-500/15 px-2 py-0.5 text-[10px] font-medium text-emerald-200">
            已就绪
          </span>
        </div>
      </button>
      {#if params.onGenerate}
        <div class="border-t border-white/10 px-5 py-2.5">
          <button
            type="button"
            class="text-xs text-indigo-300/90 hover:text-indigo-200 disabled:opacity-50"
            disabled={params.generateDisabled}
            onclick={params.onGenerate}
          >
            {params.regenerateLabel ?? params.generateLabel ?? "重新生成"}
          </button>
        </div>
      {/if}
    </div>
  {:else}
    <div class="flex flex-col rounded-2xl border border-dashed border-white/10 bg-white/[0.01] p-5">
      <div class="flex items-start gap-3">
        <span
          class="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-white/5 text-lg text-white/30"
          aria-hidden="true"
        >{params.icon}</span>
        <div class="min-w-0 flex-1">
          <p class="text-sm font-semibold text-white/75">{params.title}</p>
          <p class="mt-0.5 text-xs leading-relaxed text-white/40">{params.subtitle}</p>
        </div>
      </div>
      {#if params.onGenerate}
        <button
          type="button"
          class="mt-4 self-start rounded-lg bg-indigo-600/80 px-3 py-1.5 text-xs font-medium hover:bg-indigo-500 disabled:opacity-50"
          disabled={params.generateDisabled}
          onclick={params.onGenerate}
        >
          {params.generateLabel ?? "立即生成"}
        </button>
      {/if}
    </div>
  {/if}
{/snippet}

<div class="flex h-full flex-col overflow-y-auto bg-[#0c0e12]">
  {#if loading}
    <div class="flex flex-1 flex-col items-center justify-center gap-3 text-sm text-white/40">
      <span class="inline-block h-8 w-8 animate-spin rounded-full border-2 border-indigo-400 border-t-transparent"></span>
      <span>加载项目概览…</span>
    </div>
  {:else if !overview}
    <div class="flex flex-1 flex-col items-center justify-center gap-4 px-8 py-10 text-center">
      <div class="w-full max-w-lg rounded-2xl border border-white/10 bg-white/[0.03] p-8">
        <p class="text-xl font-semibold text-white/85">欢迎使用 MindMesh</p>
        <p class="mt-2 text-sm text-white/45">
          添加本地仓库后将自动完成索引与知识资产生成，可在本页查看状态并进入知识库阅读。
        </p>
      </div>

      {#if staleProjects.length > 0}
        <div class="w-full max-w-lg space-y-3 text-left">
          <p class="text-xs font-semibold uppercase tracking-wider text-white/40">
            检测到知识库数据丢失
          </p>
          {#each staleProjects as stale}
            {@render initBanner(
              stale.repo_path,
              stale.slug,
              "仓库 `.mind-mesh` 已缺失或损坏，可一键重新扫描并生成知识资产。",
              stale.slug,
            )}
          {/each}
        </div>
      {:else}
        <p class="max-w-md text-xs text-white/35">
          通过顶部项目选择器添加本地仓库；若索引失败，添加后可在本页初始化。
        </p>
      {/if}
    </div>
  {:else}
    <div class="mx-auto w-full max-w-6xl space-y-6 px-6 py-8">
      {#if freshness?.overall_stale && overview.repo_path}
        <div class="rounded-2xl border border-rose-500/25 bg-rose-500/[0.06] px-5 py-4">
          <div class="flex flex-wrap items-start justify-between gap-4">
            <div class="min-w-0">
              <p class="text-sm font-semibold text-rose-100/95">知识可能已过期</p>
              <p class="mt-1 text-xs leading-relaxed text-white/50">
                新鲜度 {freshness.overall_score}/100
                {#if freshness.commits_since_baseline > 0}
                  · 落后 {freshness.commits_since_baseline} 个提交
                {/if}
                {#if freshness.changed_files_count > 0}
                  · {freshness.changed_files_count} 个文件已变更
                {/if}
                {#if freshness.working_tree_dirty}
                  · 工作区有未提交修改
                {/if}
              </p>
              <p class="mt-2 text-[11px] text-rose-200/70">
                过期架构知识可能误导 Agent 问答。建议运行「快速保鲜」更新源码索引与 Agent 知识资产（跳过 Litho）。
              </p>
            </div>
            {#if onQuickRefresh}
              <button
                type="button"
                class="shrink-0 rounded-xl bg-rose-600 px-4 py-2 text-sm font-medium hover:bg-rose-500 disabled:opacity-50"
                disabled={quickRefreshBusy || initBusy}
                onclick={onQuickRefresh}
              >
                {quickRefreshBusy ? "保鲜中…" : "快速保鲜"}
              </button>
            {/if}
          </div>
        </div>
      {/if}
      {#if needsAssetInit && overview.repo_path}
        {@render initBanner(
          overview.repo_path,
          "部分知识资产尚未就绪",
          `当前 ${readyCount}/${overview.asset_health.length} 项就绪。可一键完成扫描、源码索引、${TERMS.agentKnowledge} 与 ${TERMS.humanKnowledge}。`,
          overview.slug,
        )}
      {/if}
      {#if needsEnvSetup && onOpenEnv}
        <div class="rounded-2xl border border-violet-500/20 bg-violet-500/[0.06] px-5 py-4">
          <p class="text-sm font-medium text-violet-100/90">Agent 友好的工程环境尚未配置</p>
          <p class="mt-1 text-xs text-white/45">
            为 Coding Agent 集成 Skills、工具链与 AGENTS.md（与「{TERMS.humanKnowledge}」生成无关）。
            当前 {overview.agent_env.summary}。
          </p>
          <button
            type="button"
            class="mt-3 rounded-xl bg-violet-600 px-4 py-2 text-sm font-medium hover:bg-violet-500"
            onclick={onOpenEnv}
          >
            前往 Agent 友好的工程环境 →
          </button>
        </div>
      {/if}
      <!-- Project header card -->
      <section class="overflow-hidden rounded-2xl border border-white/10 bg-white/[0.02]">
        <div class="border-b border-white/10 bg-gradient-to-r from-indigo-600/10 via-transparent to-violet-600/10 px-6 py-5">
          <div class="flex flex-wrap items-start justify-between gap-4">
            <div class="min-w-0">
              <p class="text-[10px] font-semibold uppercase tracking-widest text-indigo-300/70">项目</p>
              <h2 class="mt-1 break-words text-2xl font-bold tracking-tight text-white/95">
                {overview.name}
              </h2>
              <p class="mt-1 font-mono text-xs text-white/35">{overview.slug}</p>
            </div>
            <div class="flex flex-wrap gap-2">
              <button
                type="button"
                class="rounded-xl bg-indigo-600 px-4 py-2 text-sm font-medium hover:bg-indigo-500"
                onclick={onOpenAsk}
              >
                提问 Ask
              </button>
              {#if overview.repo_path && onOpenPath}
                <button
                  type="button"
                  class="rounded-xl border border-white/15 px-4 py-2 text-sm hover:bg-white/5"
                  onclick={() => onOpenPath(overview.repo_path)}
                >
                  打开仓库
                </button>
              {/if}
            </div>
          </div>
        </div>

        <div class="grid gap-3 p-4 sm:grid-cols-2 lg:grid-cols-3">
          {@render pathField("仓库路径", overview.repo_path)}
          {@render pathField("知识库路径", knowledgePath)}
          <div class="flex flex-col gap-1 rounded-xl border border-white/10 bg-black/20 px-3 py-2.5">
            <span class="text-[10px] font-semibold uppercase tracking-wider text-white/35">最后同步</span>
            <span class="text-sm text-white/70">{formatSyncedAt(overview.synced_at)}</span>
            {#if overview.collectors.length}
              <span class="truncate text-[10px] text-white/30" title={overview.collectors.join(", ")}>
                {overview.collectors.join(" · ")}
              </span>
            {/if}
          </div>
          <div class="flex flex-col gap-1 rounded-xl border border-white/10 bg-black/20 px-3 py-2.5">
            <span class="text-[10px] font-semibold uppercase tracking-wider text-white/35">知识资产</span>
            <span class="text-sm font-medium text-emerald-200/90">
              {readyCount} / {overview.asset_health.length}
            </span>
            <span class="text-[10px] text-white/30">就绪</span>
          </div>
          <div class="flex flex-col gap-1 rounded-xl border border-white/10 bg-black/20 px-3 py-2.5">
            <div class="flex items-center gap-1.5">
              <span class="text-[10px] font-semibold uppercase tracking-wider text-white/35">知识新鲜度</span>
              {#if freshness}
                <button
                  type="button"
                  class="inline-flex h-4 w-4 items-center justify-center rounded-full border border-white/15 text-[10px] leading-none text-white/45 transition-colors hover:border-indigo-400/50 hover:bg-indigo-500/10 hover:text-indigo-200"
                  title="了解新鲜度如何计算及本项目的偏离原因"
                  aria-label="知识新鲜度说明"
                  onclick={() => (freshnessHelpOpen = true)}
                >
                  ?
                </button>
              {/if}
            </div>
            {#if freshness}
              <div class="flex items-baseline gap-1.5">
                <span class={`text-sm font-medium ${freshnessTone(freshness.overall_score)}`}>
                  {freshness.overall_score}/100
                </span>
                {#if freshness.overall_stale}
                  <span class="text-[10px] text-amber-200/80">偏低</span>
                {/if}
              </div>
              <span class="text-[10px] text-white/30">
                {#if freshness.current_git_head}
                  HEAD {freshness.current_git_head}
                  {#if freshness.working_tree_dirty}
                    · 工作区未提交
                  {/if}
                {:else if !freshness.is_git_repo}
                  非 Git 仓库
                {:else}
                  —
                {/if}
              </span>
            {:else}
              <span class="text-sm text-white/45">—</span>
            {/if}
          </div>
          <div class="flex flex-col gap-1 rounded-xl border border-white/10 bg-black/20 px-3 py-2.5">
            <span class="text-[10px] font-semibold uppercase tracking-wider text-white/35">Agent 友好的工程环境</span>
            <span
              class={`text-sm font-medium ${overview.agent_env.ready ? "text-emerald-200/90" : "text-violet-200/80"}`}
            >
              {overview.agent_env.summary}
            </span>
            <span class="text-[10px] text-white/30">
              {overview.agent_env.ready ? "已配置" : "待集成"}
            </span>
          </div>
        </div>
      </section>

      <!-- Quick navigation cards -->
      <section>
        <h3 class="mb-3 text-xs font-semibold uppercase tracking-wider text-white/40">快速进入</h3>
        <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {@render navCard({
            title: TERMS.agentKnowledge,
            subtitle: "模块地图、核心流程与技术选型，供 Agent 与问答使用",
            ready: overview.agent_context.ready,
            meta: overview.agent_context.ready
              ? `${overview.agent_context.section_count} 个章节`
              : "尚未生成",
            icon: "🧭",
            onClick: onOpenArchitectureDoc,
            onGenerate: onGenerateAgentContext,
            generateLabel: generateLabel(TERMS.agentKnowledge, agentContextBusy),
            regenerateLabel: agentContextBusy ? "生成中…" : "重新生成",
            generateDisabled: agentContextBusy || !llmReady,
          })}
          {@render navCard({
            title: TERMS.humanKnowledge,
            subtitle: "Litho C4 文档，从 1.概述 开始阅读",
            ready: overview.litho.human_docs_complete,
            meta: overview.litho.human_docs_complete
              ? `${overview.litho.human_doc_count} 篇文档`
              : overview.litho.has_human_docs
                ? `${overview.litho.human_doc_count} 篇（未完成）`
                : overview.litho.has_research_artifacts
                  ? "研究稿已就绪，待编排"
                  : "尚未生成",
            icon: "📘",
            onClick: onOpenHumanOverview,
            onGenerate: onGenerateHuman,
            generateLabel: generateLabel(TERMS.humanKnowledge, lithoBusy),
            generateDisabled: lithoBusy || !acpOk,
          })}
          {@render navCard({
            title: TERMS.agentEnv,
            subtitle: "Skills、工具链与 AGENTS.md，供 Coding Agent 使用",
            ready: overview.agent_env.ready,
            meta: overview.agent_env.ready
              ? overview.agent_env.summary
              : "尚未配置",
            icon: "🛠",
            onClick: onOpenEnv,
            onGenerate: onOpenEnv,
            generateLabel: "前往集成",
            regenerateLabel: "管理集成",
            generateDisabled: !onOpenEnv,
          })}
        </div>
      </section>

      <!-- Asset health -->
      <section>
        <div class="mb-3 flex items-center justify-between gap-2">
          <h3 class="text-xs font-semibold uppercase tracking-wider text-white/40">知识资产</h3>
          <button
            type="button"
            class="text-xs text-indigo-300/90 hover:text-indigo-200"
            onclick={onOpenKnowledge}
          >
            进入{TERMS.knowledgeTab} →
          </button>
        </div>
        <div class="grid gap-3 sm:grid-cols-2">
          {#each overview.asset_health as asset}
            <div
              class={`rounded-xl border px-4 py-3 ${
                asset.ready
                  ? "border-emerald-500/15 bg-emerald-500/[0.03]"
                  : "border-white/10 bg-white/[0.02]"
              }`}
            >
              <div class="flex items-center justify-between gap-2">
                <div class="min-w-0">
                  <p class="text-sm font-medium text-white/85">{asset.label}</p>
                  <p class="mt-0.5 truncate text-xs {asset.ready ? 'text-emerald-200/80' : 'text-white/45'}">
                    {asset.summary}
                  </p>
                </div>
                <div class="flex shrink-0 flex-col items-end gap-1">
                  {#if asset.freshness_score != null}
                    <span
                      class={`rounded-full px-2 py-0.5 text-[10px] font-medium ${freshnessBadgeClass(asset.freshness_score, asset.stale)}`}
                    >
                      新鲜度 {asset.freshness_score}
                    </span>
                  {/if}
                  <span
                    class={`rounded-full px-2 py-0.5 text-[10px] font-medium ${
                      asset.ready
                        ? "bg-emerald-500/15 text-emerald-200"
                        : "bg-amber-500/10 text-amber-200/90"
                    }`}
                  >
                    {asset.ready ? "就绪" : "待生成"}
                  </span>
                </div>
              </div>
              <div class="mt-2 flex flex-wrap gap-3">
                {#if asset.track === "human"}
                  {#if asset.ready && onOpenHumanOverview}
                    <button
                      type="button"
                      class="text-[11px] text-indigo-300/80 hover:text-indigo-200"
                      onclick={onOpenHumanOverview}
                    >
                      浏览 →
                    </button>
                  {:else if onGenerateHuman}
                    <button
                      type="button"
                      class="text-[11px] text-indigo-300/80 hover:text-indigo-200 disabled:opacity-50"
                      disabled={lithoBusy || !acpOk}
                      onclick={onGenerateHuman}
                    >
                      {generateLabel(TERMS.humanKnowledge, lithoBusy)}
                    </button>
                  {/if}
                {:else if asset.track === "agent_context"}
                  {#if asset.ready && onOpenArchitectureDoc}
                    <button
                      type="button"
                      class="text-[11px] text-indigo-300/80 hover:text-indigo-200"
                      onclick={onOpenArchitectureDoc}
                    >
                      浏览 →
                    </button>
                  {/if}
                  {#if onGenerateAgentContext}
                    <button
                      type="button"
                      class="text-[11px] text-indigo-300/80 hover:text-indigo-200 disabled:opacity-50"
                      disabled={agentContextBusy || !llmReady}
                      onclick={onGenerateAgentContext}
                    >
                      {agentContextBusy
                        ? "生成中…"
                        : asset.ready
                          ? "重新生成"
                          : generateLabel(TERMS.agentKnowledge, false)}
                    </button>
                  {/if}
                {:else if asset.track === "agent_pack" && onRepack}
                  <button
                    type="button"
                    class="text-[11px] text-indigo-300/80 hover:text-indigo-200 disabled:opacity-50"
                    disabled={repackBusy}
                    onclick={onRepack}
                  >
                    {repackBusy ? "Repacking…" : "Repack"}
                  </button>
                {:else if asset.track === "structured"}
                  {#if asset.ready && onOpenStructured}
                    <button
                      type="button"
                      class="text-[11px] text-indigo-300/80 hover:text-indigo-200"
                      onclick={onOpenStructured}
                    >
                      浏览 →
                    </button>
                  {:else}
                    <button
                      type="button"
                      class="text-[11px] text-indigo-300/80 hover:text-indigo-200"
                      onclick={onOpenKnowledge}
                    >
                      扫描项目
                    </button>
                  {/if}
                {/if}
              </div>
            </div>
          {/each}
        </div>
      </section>

      <!-- Footer links -->
      <section class="flex flex-wrap items-center justify-between gap-3 border-t border-white/10 pt-4">
        <div class="flex flex-wrap gap-2">
          <button
            type="button"
            class="rounded-lg border border-white/10 px-3 py-1.5 text-xs hover:bg-white/5"
            onclick={onOpenKnowledge}
          >
            知识资产
          </button>
          <button
            type="button"
            class="rounded-lg border border-white/10 px-3 py-1.5 text-xs hover:bg-white/5"
            onclick={onOpenSdd}
          >
            SDD 工作流
          </button>
        </div>
        <p class="text-[11px] text-white/35">
          LLM {llmReady ? "✓" : "✗"} · ACP {acpOk ? "✓" : "✗"}
          {#if !llmReady}
            <span class="text-amber-300/70"> · 请在设置中配置 LLM</span>
          {/if}
        </p>
      </section>
    </div>
  {/if}
</div>

<FreshnessHelpPanel
  open={freshnessHelpOpen}
  freshness={freshness}
  quickRefreshBusy={quickRefreshBusy}
  onQuickRefresh={onQuickRefresh}
  onclose={() => (freshnessHelpOpen = false)}
/>
