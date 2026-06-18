<script lang="ts">
  import type { ProjectOverview, StaleProjectSummary } from "../types";
  import { generateLabel, SHORT_TERMS, TERMS, UI_MESSAGES } from "../terminology";
  import FreshnessHelpPanel from "./FreshnessHelpPanel.svelte";
  import OverviewActionBanner, {
    type OverviewActionItem,
  } from "./OverviewActionBanner.svelte";
  import OverviewKnowledgeCard from "./OverviewKnowledgeCard.svelte";
  import ReadinessHelpPanel from "./ReadinessHelpPanel.svelte";

  interface Props {
    overview: ProjectOverview | null;
    loading: boolean;
    acpOk: boolean;
    llmReady: boolean;
    hybridNativeLlm?: boolean;
    agentContextBusy?: boolean;
    lithoBusy?: boolean;
    repackBusy?: boolean;
    initBusy?: boolean;
    initProgress?: string | null;
    staleProjects?: StaleProjectSummary[];
    onOpenKnowledge: () => void;
    onOpenEnv?: () => void;
    onOpenAsk: () => void;
    onOpenSettings?: () => void;
    onGenerateHuman?: () => void;
    onGenerateAgentContext?: () => void;
    onRepack?: () => void;
    onInitializeProject?: (repoPath: string, slug?: string) => void;
    onOpenPath?: (path: string) => void;
    onOpenArchitectureDoc?: () => void;
    onOpenHumanOverview?: () => void;
    onOpenStructured?: () => void;
    quickRefreshBusy?: boolean;
    freshnessLoading?: boolean;
    onQuickRefresh?: () => void;
    onSaveProjectRemark?: (remark: string) => Promise<void>;
  }

  let {
    overview,
    loading,
    acpOk,
    llmReady,
    hybridNativeLlm = false,
    agentContextBusy = false,
    lithoBusy = false,
    repackBusy = false,
    initBusy = false,
    initProgress = null,
    staleProjects = [],
    onOpenKnowledge,
    onOpenEnv,
    onOpenAsk,
    onOpenSettings,
    onGenerateHuman,
    onGenerateAgentContext,
    onRepack,
    onInitializeProject,
    onOpenPath,
    onOpenArchitectureDoc,
    onOpenHumanOverview,
    onOpenStructured,
    quickRefreshBusy = false,
    freshnessLoading = false,
    onQuickRefresh,
    onSaveProjectRemark,
  }: Props = $props();

  let freshnessHelpOpen = $state(false);
  let readinessHelpOpen = $state(false);
  let remarkEditing = $state(false);
  let remarkDraft = $state("");
  let remarkSaving = $state(false);

  const readyCount = $derived(
    overview?.asset_health.filter((a) => a.ready).length ?? 0,
  );

  const assetTotal = $derived(overview?.asset_health.length ?? 0);

  const readinessPercent = $derived(
    assetTotal > 0 ? Math.round((readyCount / assetTotal) * 100) : 0,
  );

  const needsAssetInit = $derived(
    overview != null && readyCount < assetTotal,
  );

  const needsEnvSetup = $derived(overview != null && !overview.agent_env.ready);

  const initHint = $derived.by(() => {
    const needsLlm = hybridNativeLlm && !llmReady;
    const needsAcp = !acpOk;
    if (!needsLlm && !needsAcp) return null;
    const parts: string[] = [];
    if (needsLlm) parts.push(`LLM（${TERMS.agentKnowledge}）`);
    if (needsAcp) parts.push(`ACP（${TERMS.humanKnowledge}）`);
    return `部分步骤需要配置：${parts.join("、")}，可在设置中完成后再试`;
  });

  const knowledgePath = $derived(
    overview?.repo_path ? `${overview.repo_path}/.mind-mesh` : null,
  );

  const freshness = $derived(overview?.freshness ?? null);

  const freshnessScore = $derived(freshness?.overall_score ?? null);

  const needsGenerationSetup = $derived(
    (hybridNativeLlm && !llmReady) || !acpOk,
  );

  const actionItems = $derived.by((): OverviewActionItem[] => {
    if (!overview) return [];

    const items: OverviewActionItem[] = [];

    if (freshness?.overall_stale && overview.repo_path && onQuickRefresh) {
      const driftParts: string[] = [`新鲜度 ${freshness.overall_score}/100`];
      if (freshness.commits_since_baseline > 0) {
        driftParts.push(`落后 ${freshness.commits_since_baseline} 个提交`);
      }
      if (freshness.changed_files_count > 0) {
        driftParts.push(`${freshness.changed_files_count} 个文件已变更`);
      }
      if (freshness.working_tree_dirty) {
        driftParts.push("工作区有未提交修改");
      }
      items.push({
        id: "stale",
        priority: 1,
        accent: "rose",
        title: "知识可能已过期",
        detail: `${driftParts.join(" · ")}。过期架构知识可能误导 Agent 问答。`,
        hint: "建议运行「快速保鲜」更新源码索引与 Agent 知识资产（跳过 Litho）。",
        actionLabel: "快速保鲜",
        busyLabel: "保鲜中…",
        onAction: onQuickRefresh,
        disabled: initBusy,
        busy: quickRefreshBusy,
      });
    }

    if (needsAssetInit && overview.repo_path && onInitializeProject) {
      items.push({
        id: "init",
        priority: 2,
        accent: "amber",
        title: "部分知识资产尚未就绪",
        detail: `当前 ${readyCount}/${assetTotal} 项就绪。可一键完成扫描、源码索引、${TERMS.agentKnowledge} 与 ${TERMS.humanKnowledge}。`,
        hint: initHint ?? undefined,
        actionLabel: "一键初始化",
        busyLabel: initProgress ?? "初始化中…",
        onAction: () => onInitializeProject(overview.repo_path, overview.slug),
        disabled: initBusy,
        busy: initBusy,
      });
    }

    if (needsEnvSetup && onOpenEnv) {
      items.push({
        id: "env",
        priority: 3,
        accent: "violet",
        title: `${TERMS.agentEnv}尚未配置`,
        detail: `为 Coding Agent 集成 Skills、工具链与 AGENTS.md。当前 ${overview.agent_env.summary}。`,
        actionLabel: "前往配置",
        onAction: onOpenEnv,
      });
    }

    return items;
  });

  const staleActionItems = $derived.by((): OverviewActionItem[] =>
    staleProjects.map((stale) => ({
      id: `stale-${stale.slug}`,
      priority: 2,
      accent: "amber" as const,
      title: stale.slug,
      detail: "仓库 `.mind-mesh` 已缺失或损坏，可一键重新扫描并生成知识资产。",
      hint: initHint ?? undefined,
      actionLabel: "重新初始化",
      busyLabel: initProgress ?? "初始化中…",
      onAction: onInitializeProject
        ? () => onInitializeProject(stale.repo_path, stale.slug)
        : undefined,
      disabled: initBusy || !onInitializeProject,
      busy: initBusy,
    })),
  );

  function freshnessBarClass(score: number): string {
    if (score >= 80) return "bg-emerald-500";
    if (score >= 50) return "bg-amber-500";
    return "bg-rose-500";
  }

  function freshnessTextClass(score: number): string {
    if (score >= 80) return "text-emerald-200";
    if (score >= 50) return "text-amber-200";
    return "text-rose-200";
  }

  function freshnessBadgeClass(score: number, stale?: boolean | null): string {
    if (!stale && score >= 80) return "bg-emerald-500/15 text-emerald-200";
    if (score >= 50) return "bg-amber-500/15 text-amber-200";
    return "bg-rose-500/15 text-rose-200";
  }

  function readinessBarClass(percent: number): string {
    if (percent >= 100) return "bg-emerald-500";
    if (percent >= 50) return "bg-amber-500";
    return "bg-rose-500";
  }

  function formatSyncedAt(value?: string | null): string {
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

  function humanKnowledgeMeta(o: ProjectOverview): string {
    if (o.litho.human_docs_complete) {
      return `${o.litho.human_doc_count} 篇文档`;
    }
    if (o.litho.has_human_docs) {
      return `${o.litho.human_doc_count} 篇（未完成）`;
    }
    if (o.litho.has_research_artifacts) {
      return "研究稿已就绪，待编排";
    }
    return "尚未生成";
  }

  function assetPrimaryAction(asset: ProjectOverview["asset_health"][number]): {
    label: string;
    onClick?: () => void;
    disabled?: boolean;
  } | null {
    if (asset.track === "human") {
      if (asset.ready && onOpenHumanOverview) {
        return { label: "浏览", onClick: onOpenHumanOverview };
      }
      if (onGenerateHuman) {
        return {
          label: generateLabel(TERMS.humanKnowledge, lithoBusy),
          onClick: onGenerateHuman,
          disabled: lithoBusy || !acpOk,
        };
      }
    } else if (asset.track === "agent_context") {
      if (asset.ready && onOpenArchitectureDoc) {
        return { label: "浏览", onClick: onOpenArchitectureDoc };
      }
      if (onGenerateAgentContext) {
        return {
          label: asset.ready
            ? agentContextBusy
              ? "生成中…"
              : "重新生成"
            : generateLabel(TERMS.agentKnowledge, false),
          onClick: onGenerateAgentContext,
          disabled: agentContextBusy || !llmReady,
        };
      }
    } else if (asset.track === "agent_pack" && onRepack) {
      return {
        label: repackBusy ? UI_MESSAGES.repacking : UI_MESSAGES.repack,
        onClick: onRepack,
        disabled: repackBusy,
      };
    } else if (asset.track === "structured") {
      if (asset.ready && onOpenStructured) {
        return { label: "浏览", onClick: onOpenStructured };
      }
      return { label: "扫描项目", onClick: onOpenKnowledge };
    }
    return null;
  }

  const assetRows = $derived(
    overview?.asset_health.map((asset) => ({
      asset,
      action: assetPrimaryAction(asset),
    })) ?? [],
  );

  function startRemarkEdit() {
    remarkDraft = overview?.project_remark ?? "";
    remarkEditing = true;
  }

  function cancelRemarkEdit() {
    remarkEditing = false;
    remarkDraft = overview?.project_remark ?? "";
  }

  async function saveRemark() {
    if (!onSaveProjectRemark) return;
    remarkSaving = true;
    try {
      await onSaveProjectRemark(remarkDraft);
      remarkEditing = false;
    } finally {
      remarkSaving = false;
    }
  }
</script>

{#snippet pathRow(label: string, path: string)}
  <button
    type="button"
    class="group flex w-full items-center justify-between gap-3 rounded-lg border border-white/8 bg-black/20 px-3 py-2.5 text-left transition-colors hover:border-indigo-500/30 hover:bg-indigo-500/[0.06]"
    onclick={() => onOpenPath?.(path)}
    title={`在 Finder 中打开：${path}`}
  >
    <div class="min-w-0">
      <span class="text-xs text-white/40">{label}</span>
      <p class="truncate font-mono text-xs text-indigo-200/90 group-hover:text-indigo-100">
        {path}
      </p>
    </div>
    <span class="shrink-0 text-xs text-white/25 group-hover:text-indigo-300">打开</span>
  </button>
{/snippet}

<div class="flex h-full flex-col overflow-y-auto bg-[#0c0e12]">
  {#if loading}
    <div class="flex flex-1 flex-col items-center justify-center gap-3 text-sm text-white/40">
      <span
        class="inline-block h-8 w-8 animate-spin rounded-full border-2 border-indigo-400 border-t-transparent"
      ></span>
      <span>加载项目概览…</span>
    </div>
  {:else if !overview}
    <div class="mx-auto flex w-full max-w-3xl flex-1 flex-col justify-center gap-6 px-6 py-10">
      <section class="rounded-2xl border border-white/8 bg-[#14171c] px-6 py-8">
        <h2 class="text-xl font-semibold text-white/90">欢迎使用 MindMesh</h2>
        <p class="mt-2 text-sm leading-relaxed text-white/45">
          添加本地仓库后将自动完成索引与知识资产生成，可在本页查看状态并进入知识库阅读。
        </p>
        {#if staleProjects.length === 0}
          <p class="mt-4 text-xs text-white/35">
            通过顶部项目选择器添加本地仓库；若索引失败，添加后可在本页初始化。
          </p>
        {/if}
      </section>

      {#if staleProjects.length > 0}
        <section class="space-y-3">
          <h3 class="text-sm font-medium text-white/55">检测到知识库数据丢失</h3>
          <OverviewActionBanner
            items={staleActionItems}
            progressNote={initBusy ? initProgress : null}
          />
        </section>
      {/if}
    </div>
  {:else}
    <div class="mx-auto w-full max-w-6xl space-y-6 px-6 py-8">
      <!-- Project header -->
      <header class="flex flex-wrap items-start justify-between gap-4">
        <div class="min-w-0 flex-1">
          <h2 class="break-words text-xl font-semibold tracking-tight text-white/95">
            {overview.name}
          </h2>

          {#if remarkEditing}
            <div class="mt-2 space-y-2">
              <textarea
                class="w-full resize-y rounded-xl border border-white/15 bg-black/25 px-3 py-2 text-sm text-white/80 placeholder:text-white/30 focus:border-indigo-500/40 focus:outline-none"
                rows="2"
                placeholder="填写项目备注，将保存至 .mind-mesh/project-note.md"
                bind:value={remarkDraft}
                disabled={remarkSaving}
              ></textarea>
              <div class="flex flex-wrap gap-2">
                <button
                  type="button"
                  class="rounded-lg bg-indigo-600 px-3 py-1.5 text-xs font-medium hover:bg-indigo-500 disabled:opacity-50"
                  disabled={remarkSaving || !onSaveProjectRemark}
                  onclick={saveRemark}
                >
                  {remarkSaving ? "保存中…" : "保存备注"}
                </button>
                <button
                  type="button"
                  class="rounded-lg border border-white/15 px-3 py-1.5 text-xs text-white/60 hover:bg-white/5"
                  disabled={remarkSaving}
                  onclick={cancelRemarkEdit}
                >
                  取消
                </button>
              </div>
            </div>
          {:else}
            <div class="mt-1 flex items-start gap-2">
              {#if overview.project_remark}
                <p class="text-sm leading-relaxed text-white/55">{overview.project_remark}</p>
              {:else}
                <p class="text-sm text-white/30">添加项目备注，便于团队识别此仓库</p>
              {/if}
              {#if onSaveProjectRemark}
                <button
                  type="button"
                  class="shrink-0 rounded-md border border-white/10 px-2 py-0.5 text-[11px] text-white/45 hover:bg-white/5 hover:text-white/70"
                  onclick={startRemarkEdit}
                >
                  编辑
                </button>
              {/if}
            </div>
          {/if}

          <p class="mt-2 text-xs text-white/40">
            最后同步 {formatSyncedAt(overview.synced_at)}
            {#if overview.collectors.length}
              <span class="text-white/25"> · {overview.collectors.join(" · ")}</span>
            {/if}
            <span class="text-white/25"> · </span>
            <span class="font-mono text-white/30">{overview.slug}</span>
          </p>
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
          {#if freshness?.overall_stale && onQuickRefresh}
            <button
              type="button"
              class="rounded-xl border border-rose-500/30 px-4 py-2 text-sm text-rose-200/90 hover:bg-rose-500/10 disabled:opacity-50"
              disabled={quickRefreshBusy || initBusy}
              onclick={onQuickRefresh}
            >
              {quickRefreshBusy ? "保鲜中…" : "快速保鲜"}
            </button>
          {/if}
        </div>
      </header>

      {#if actionItems.length > 0}
        <OverviewActionBanner
          items={actionItems}
          progressNote={initBusy && !actionItems.some((i) => i.id === "init") ? initProgress : null}
        />
      {/if}

      <!-- 知识资产域 -->
      <section class="space-y-3">
        <div>
          <h3 class="text-sm font-medium text-white/70">知识资产</h3>
          <p class="mt-0.5 text-xs text-white/40">仓库内 <code class="text-white/55">.mind-mesh/</code> 的完整性与时效</p>
        </div>

        <div class="space-y-5 rounded-2xl border border-white/8 bg-[#14171c] p-5">
          <div class="grid gap-5 sm:grid-cols-2">
            <div>
              <div class="flex items-baseline justify-between gap-2">
                <div class="flex items-center gap-1.5">
                  <span class="text-xs text-white/40">就绪度</span>
                  <button
                    type="button"
                    class="inline-flex h-4 w-4 items-center justify-center rounded-full border border-white/15 text-[10px] text-white/45 hover:border-indigo-400/50 hover:text-indigo-200"
                    title="查看各项知识资产就绪情况"
                    aria-label="就绪度说明"
                    onclick={() => (readinessHelpOpen = true)}
                  >
                    ?
                  </button>
                </div>
                <span class="text-2xl font-semibold tabular-nums text-white/90">
                  {readyCount}<span class="text-base font-normal text-white/35">/{assetTotal}</span>
                </span>
              </div>
              <div class="mt-2 h-2 overflow-hidden rounded-full bg-white/8">
                <div
                  class={`h-full rounded-full transition-all ${readinessBarClass(readinessPercent)}`}
                  style={`width: ${readinessPercent}%`}
                ></div>
              </div>
            </div>

            <div>
              <div class="flex items-center justify-between gap-2">
                <div class="flex items-center gap-1.5">
                  <span class="text-xs text-white/40">新鲜度</span>
                  <button
                    type="button"
                    class="inline-flex h-4 w-4 items-center justify-center rounded-full border border-white/15 text-[10px] text-white/45 hover:border-indigo-400/50 hover:text-indigo-200"
                    title="了解新鲜度如何计算及本项目的偏离原因"
                    aria-label="知识新鲜度说明"
                    onclick={() => (freshnessHelpOpen = true)}
                  >
                    ?
                  </button>
                </div>
                {#if freshnessLoading && !freshness}
                  <span class="text-xs text-white/35">计算中…</span>
                {:else if freshnessScore != null}
                  <span class={`text-2xl font-semibold tabular-nums ${freshnessTextClass(freshnessScore)}`}>
                    {freshnessScore}
                    <span class="text-base font-normal text-white/35">/100</span>
                  </span>
                {:else}
                  <span class="text-sm text-white/45">—</span>
                {/if}
              </div>

              {#if freshnessLoading && !freshness}
                <div class="mt-2 h-2 animate-pulse rounded-full bg-white/10" role="status" aria-live="polite"></div>
              {:else if freshnessScore != null}
                <div class="mt-2 h-2 overflow-hidden rounded-full bg-white/8">
                  <div
                    class={`h-full rounded-full transition-all ${freshnessBarClass(freshnessScore)}`}
                    style={`width: ${freshnessScore}%`}
                  ></div>
                </div>
                <p class="mt-1.5 text-[11px] text-white/30">
                  {#if freshnessLoading}
                    更新中…
                  {:else if freshness?.is_git_repo && freshness.current_git_head}
                    基于 Git 提交对比 · HEAD {freshness.current_git_head}
                    {#if freshness.working_tree_dirty}
                      · 工作区有未提交修改
                    {/if}
                  {:else if freshness && !freshness.is_git_repo}
                    未检测到 Git，分数按知识资产同步时间估算
                  {/if}
                </p>
              {/if}
            </div>
          </div>

          <div class="border-t border-white/8 pt-5">
            <h4 class="text-xs font-medium text-white/55">阅读与生成</h4>
            <div class="mt-3 grid gap-3 sm:grid-cols-2">
              <OverviewKnowledgeCard
                nested
                title={SHORT_TERMS.agentKnowledge}
                subtitle="模块地图、核心流程与技术选型，供 Agent 与问答使用"
                meta={overview.agent_context.ready
                  ? `${overview.agent_context.section_count} 个章节`
                  : "尚未生成"}
                ready={overview.agent_context.ready}
                icon="compass"
                primaryLabel={overview.agent_context.ready
                  ? "打开"
                  : generateLabel(TERMS.agentKnowledge, agentContextBusy)}
                onPrimary={overview.agent_context.ready ? onOpenArchitectureDoc : onGenerateAgentContext}
                primaryDisabled={overview.agent_context.ready
                  ? !onOpenArchitectureDoc
                  : agentContextBusy || !llmReady || !onGenerateAgentContext}
                secondaryLabel={overview.agent_context.ready ? (agentContextBusy ? "生成中…" : "重新生成") : undefined}
                onSecondary={overview.agent_context.ready ? onGenerateAgentContext : undefined}
                secondaryDisabled={agentContextBusy || !llmReady}
              />
              <OverviewKnowledgeCard
                nested
                title={SHORT_TERMS.humanKnowledge}
                subtitle="Litho C4 文档，从 1.概述 开始阅读"
                meta={humanKnowledgeMeta(overview)}
                ready={overview.litho.human_docs_complete}
                icon="book"
                primaryLabel={overview.litho.human_docs_complete ? "打开" : generateLabel(TERMS.humanKnowledge, lithoBusy)}
                onPrimary={overview.litho.human_docs_complete ? onOpenHumanOverview : onGenerateHuman}
                primaryDisabled={overview.litho.human_docs_complete
                  ? !onOpenHumanOverview
                  : lithoBusy || !acpOk || !onGenerateHuman}
              />
            </div>
          </div>

          {#if needsGenerationSetup}
            <div class="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-white/8 bg-white/[0.02] px-4 py-3">
              <p class="text-xs text-white/45">
                生成知识资产需要 MindMesh 生成能力：
                {#if hybridNativeLlm && !llmReady}
                  <span class="text-amber-200/90">LLM 未配置</span>
                {/if}
                {#if hybridNativeLlm && !llmReady && !acpOk}
                  <span class="text-white/30"> · </span>
                {/if}
                {#if !acpOk}
                  <span class="text-amber-200/90">ACP 未连接</span>
                {/if}
              </p>
              {#if onOpenSettings}
                <button
                  type="button"
                  class="shrink-0 text-xs text-indigo-300/90 hover:text-indigo-200"
                  onclick={onOpenSettings}
                >
                  前往设置
                </button>
              {/if}
            </div>
          {/if}

        </div>
      </section>

      <!-- Agent 工程环境域 -->
      <section class="space-y-3">
        <div>
          <h3 class="text-sm font-medium text-white/70">{TERMS.agentEnv}</h3>
          <p class="mt-0.5 text-xs text-white/40">Skills、工具链与 AGENTS.md，供 Coding Agent 在仓库中协作</p>
        </div>

        <div class="flex flex-wrap items-center justify-between gap-4 rounded-2xl border border-white/8 bg-[#14171c] px-5 py-4">
          <div class="min-w-0">
            <div class="flex flex-wrap items-center gap-2">
              <p class="text-sm font-medium text-white/85">{overview.agent_env.summary}</p>
              <span
                class={`rounded-full px-2 py-0.5 text-[10px] font-medium ${
                  overview.agent_env.ready
                    ? "bg-emerald-500/15 text-emerald-200"
                    : "bg-white/8 text-white/45"
                }`}
              >
                {overview.agent_env.ready ? "已集成" : "待配置"}
              </span>
            </div>
            <p class="mt-1 text-xs text-white/40">
              与知识资产生成无关，用于优化外部 Coding Agent 在本仓库的工作体验。
            </p>
          </div>
          {#if onOpenEnv}
            <button
              type="button"
              class="shrink-0 rounded-xl bg-indigo-600 px-4 py-2 text-sm font-medium hover:bg-indigo-500"
              onclick={onOpenEnv}
            >
              {overview.agent_env.ready ? "管理集成" : "前往配置"}
            </button>
          {/if}
        </div>
      </section>

      {#if overview.repo_path || knowledgePath}
        <details class="group rounded-2xl border border-white/8 bg-[#14171c]">
          <summary
            class="cursor-pointer list-none px-5 py-3.5 text-sm font-medium text-white/70 marker:content-none [&::-webkit-details-marker]:hidden"
          >
            <span class="text-white/35 group-open:hidden">▸</span>
            <span class="hidden text-white/35 group-open:inline">▾</span>
            路径信息
          </summary>
          <div class="space-y-2 border-t border-white/8 px-5 py-4">
            {#if overview.repo_path}
              {@render pathRow("仓库路径", overview.repo_path)}
            {/if}
            {#if knowledgePath}
              {@render pathRow("知识库路径", knowledgePath)}
            {/if}
          </div>
        </details>
      {/if}
    </div>
  {/if}
</div>

<FreshnessHelpPanel
  open={freshnessHelpOpen}
  {freshness}
  {quickRefreshBusy}
  onQuickRefresh={onQuickRefresh}
  onclose={() => (freshnessHelpOpen = false)}
/>

<ReadinessHelpPanel
  open={readinessHelpOpen}
  {readyCount}
  {assetTotal}
  rows={assetRows}
  {freshnessBadgeClass}
  onOpenKnowledge={onOpenKnowledge}
  onclose={() => (readinessHelpOpen = false)}
/>
