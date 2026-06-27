<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { getEnvStatus, planEnvIntegration, runEnvIntegration } from "../api";
  import type { EnvIntegrationStatus, EnvPlan, EnvStatus } from "../types";
  import { TERMS } from "../terminology";
  import EnvPlanPanel from "./EnvPlanPanel.svelte";

  interface Props {
    repoPath: string | null;
    onStatus?: (message: string, kind: "idle" | "loading" | "progress" | "success" | "error") => void;
    onIntegrated?: () => void;
  }

  let { repoPath, onStatus, onIntegrated }: Props = $props();

  let status = $state<EnvStatus | null>(null);
  let plan = $state<EnvPlan | null>(null);
  let loading = $state(false);
  let loadingHint = $state("正在读取集成清单…");
  let applying = $state(false);
  let progressMessage = $state<string | null>(null);
  let selected = $state<Set<string>>(new Set());
  let reinstallMarked = $state<Set<string>>(new Set());
  let planHelpOpen = $state(false);

  const skillItems = $derived(status?.items.filter((i) => i.kind === "skill") ?? []);
  const toolItems = $derived(status?.items.filter((i) => i.kind === "tool") ?? []);
  const configItems = $derived(
    status?.items.filter((i) => i.kind === "agents_md" || i.kind === "gitignore") ?? [],
  );

  const applyCount = $derived(plan?.steps.length ?? 0);
  const canApply = $derived(!applying && applyCount > 0);

  function isReinstallPending(item: EnvIntegrationStatus): boolean {
    return reinstallMarked.has(item.id);
  }

  function isBundledLocked(item: EnvIntegrationStatus): boolean {
    return item.locked && !isReinstallPending(item);
  }

  function isLockedIntegrated(item: EnvIntegrationStatus): boolean {
    return item.integrated && !item.locked && !isReinstallPending(item);
  }

  function isCheckboxChecked(item: EnvIntegrationStatus): boolean {
    if (isReinstallPending(item) || isBundledLocked(item) || isLockedIntegrated(item)) {
      return true;
    }
    return selected.has(item.id);
  }

  function isCheckboxDisabled(item: EnvIntegrationStatus): boolean {
    if (isReinstallPending(item) || isBundledLocked(item) || isLockedIntegrated(item)) {
      return true;
    }
    return dependencyBlocked(item);
  }

  function canMarkReinstall(item: EnvIntegrationStatus): boolean {
    return (item.integrated || item.locked) && !isReinstallPending(item);
  }

  function canCancelReinstall(item: EnvIntegrationStatus): boolean {
    return isReinstallPending(item);
  }

  function dependencySatisfied(depId: string): boolean {
    const dep = status?.items.find((i) => i.id === depId);
    if (!dep) return false;
    if (dep.integrated || dep.locked) return true;
    if (dep.integrated && !reinstallMarked.has(depId)) return true;
    return selected.has(depId);
  }

  function dependencyBlocked(item: EnvIntegrationStatus): boolean {
    if (isBundledLocked(item) || isLockedIntegrated(item)) return false;
    return item.depends_on.some((d) => !dependencySatisfied(d));
  }

  function defaultSelectedIds(items: EnvIntegrationStatus[]): Set<string> {
    return new Set(items.filter((i) => !i.integrated).map((i) => i.id));
  }

  async function loadStatus() {
    if (!repoPath) {
      status = null;
      plan = null;
      return;
    }
    loading = true;
    loadingHint = "正在检测 Skills 与工具链…";
    try {
      status = await getEnvStatus(repoPath);
      loadingHint = "正在生成本次集成计划…";
      selected = defaultSelectedIds(status.items);
      reinstallMarked = new Set();
      await refreshPlan(selected);
    } catch (e) {
      status = null;
      onStatus?.(String(e), "error");
    } finally {
      loading = false;
      loadingHint = "正在读取集成清单…";
    }
  }

  async function refreshPlan(ids: Set<string>, reinstall: Set<string> = reinstallMarked) {
    if (!repoPath) return;
    try {
      plan = await planEnvIntegration(repoPath, [...ids], [...reinstall]);
    } catch {
      plan = null;
    }
  }

  function toggle(id: string) {
    const item = status?.items.find((i) => i.id === id);
    if (
      !item ||
      isReinstallPending(item) ||
      isBundledLocked(item) ||
      isLockedIntegrated(item) ||
      dependencyBlocked(item)
    ) {
      return;
    }

    const next = new Set(selected);
    const nextReinstall = new Set(reinstallMarked);

    if (next.has(id)) {
      next.delete(id);
      nextReinstall.delete(id);
    } else {
      next.add(id);
    }

    selected = next;
    reinstallMarked = nextReinstall;
    void refreshPlan(next, nextReinstall);
  }

  function markForReinstall(id: string) {
    const item = status?.items.find((i) => i.id === id);
    if (!item || !canMarkReinstall(item)) return;

    const next = new Set(selected);
    const nextReinstall = new Set(reinstallMarked);
    next.add(id);
    nextReinstall.add(id);
    selected = next;
    reinstallMarked = nextReinstall;
    void refreshPlan(next, nextReinstall);
  }

  function cancelReinstall(id: string) {
    const item = status?.items.find((i) => i.id === id);
    if (!item || !canCancelReinstall(item)) return;

    const next = new Set(selected);
    const nextReinstall = new Set(reinstallMarked);
    next.delete(id);
    nextReinstall.delete(id);
    selected = next;
    reinstallMarked = nextReinstall;
    void refreshPlan(next, nextReinstall);
  }

  function selectAllPending() {
    if (!status) return;
    selected = defaultSelectedIds(status.items);
    reinstallMarked = new Set();
    void refreshPlan(selected);
  }

  function selectNone() {
    if (!status) return;
    selected = new Set();
    reinstallMarked = new Set();
    void refreshPlan(selected);
  }

  async function apply() {
    if (!repoPath || applyCount === 0) return;
    applying = true;
    progressMessage = "准备集成…";
    onStatus?.("正在集成 Agent 友好的工程环境…", "progress");
    try {
      const result = await runEnvIntegration(repoPath, [...selected], [...reinstallMarked]);
      if (result.errors.length > 0) {
        onStatus?.(`部分失败：${result.errors.join("; ")}`, "error");
      } else {
        onStatus?.(`已集成 ${result.applied.length} 项`, "success");
      }
      await loadStatus();
      onIntegrated?.();
    } catch (e) {
      onStatus?.(String(e), "error");
    } finally {
      applying = false;
      progressMessage = null;
    }
  }

  onMount(() => {
    const unsubs: (() => void)[] = [];
    void (async () => {
      unsubs.push(
        await listen<{ repo_path: string; message: string }>("env-opt-progress", (ev) => {
          if (repoPath && ev.payload.repo_path === repoPath) {
            progressMessage = ev.payload.message;
          }
        }),
      );
      unsubs.push(
        await listen<{ repo_path: string }>("env-opt-done", (ev) => {
          if (repoPath && ev.payload.repo_path === repoPath) {
            progressMessage = null;
          }
        }),
      );
    })();
    return () => unsubs.forEach((u) => u());
  });

  $effect(() => {
    void loadStatus();
  });
</script>

<div class="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto p-4">
  <header>
    <h2 class="text-lg font-semibold text-white/90">{TERMS.agentEnv}</h2>
    <p class="mt-1 text-sm text-white/50">
      为 Coding Agent（Claude Code、Cursor 等）配置 Skills、工具链与 AGENTS.md。
      知识资产使用仓库内 <code class="text-white/70">.terrain/</code>，由 Terrain 生成。
    </p>
    {#if status}
      <p class="mt-2 text-sm text-indigo-200/80">{status.summary}</p>
    {/if}
  </header>

  {#if !repoPath}
    <p class="text-sm text-white/45">请先选择已索引的项目。</p>
  {:else if loading}
    <div
      class="rounded-xl border border-indigo-500/15 bg-indigo-500/[0.04] px-4 py-4"
      role="status"
      aria-live="polite"
      aria-busy="true"
    >
      <div class="flex items-start gap-3">
        <div
          class="mt-0.5 h-5 w-5 shrink-0 animate-spin rounded-full border-2 border-indigo-400/30 border-t-indigo-300"
        ></div>
        <div class="min-w-0">
          <p class="text-sm font-medium text-white/80">正在检测 {TERMS.agentEnv}</p>
          <p class="mt-1 text-xs text-white/45">{loadingHint}</p>
          <p class="mt-2 text-[11px] text-white/30">
            首次打开会扫描 Skills、AGENTS.md 与 CodeGraph 索引，通常很快完成。
          </p>
        </div>
      </div>
    </div>

    {#snippet skeletonSection(title: string, rows: number)}
      <section>
        <div class="mb-2 h-3 w-20 animate-pulse rounded bg-white/10"></div>
        <div class="grid gap-2">
          {#each Array.from({ length: rows }, (_, i) => i) as i (i)}
            <div
              class="flex items-start gap-3 rounded-xl border border-white/5 bg-white/[0.02] px-4 py-3"
            >
              <div class="mt-1 h-4 w-4 shrink-0 animate-pulse rounded bg-white/10"></div>
              <div class="min-w-0 flex-1 space-y-2">
                <div class="h-3.5 w-2/5 max-w-[12rem] animate-pulse rounded bg-white/10"></div>
                <div class="h-2.5 w-full animate-pulse rounded bg-white/[0.06]"></div>
                <div class="h-2.5 w-4/5 max-w-[20rem] animate-pulse rounded bg-white/[0.05]"></div>
              </div>
            </div>
          {/each}
        </div>
      </section>
    {/snippet}

    <div class="mt-4 space-y-6 opacity-80">
      {@render skeletonSection("Skills", 4)}
      {@render skeletonSection("工具链", 3)}
      {@render skeletonSection("配置", 3)}
    </div>
  {:else if status}
    <div class="flex flex-wrap items-center gap-2">
      <button
        type="button"
        class="rounded-lg border border-white/10 px-3 py-1.5 text-xs hover:bg-white/5"
        onclick={selectAllPending}
      >
        全选待集成
      </button>
      <button
        type="button"
        class="rounded-lg border border-white/10 px-3 py-1.5 text-xs hover:bg-white/5"
        onclick={selectNone}
      >
        清空可选
      </button>
      <div class="flex items-center gap-1.5">
        <button
          type="button"
          class="rounded-lg bg-indigo-600 px-4 py-1.5 text-xs font-medium hover:bg-indigo-500 disabled:opacity-50"
          disabled={applying || applyCount === 0}
          onclick={apply}
        >
          {applying ? "集成中…" : `集成所选 (${applyCount})`}
        </button>
        {#if canApply}
          <button
            type="button"
            class="inline-flex h-6 w-6 items-center justify-center rounded-full border border-white/15 text-xs text-white/45 hover:border-indigo-400/50 hover:text-indigo-200"
            title="查看执行计划详情"
            aria-label="执行计划说明"
            onclick={() => (planHelpOpen = true)}
          >
            ?
          </button>
        {/if}
      </div>
      {#if progressMessage}
        <span class="text-xs text-white/45">{progressMessage}</span>
      {/if}
    </div>

    {#snippet itemRow(item: EnvIntegrationStatus)}
      {@const bundledLocked = isBundledLocked(item)}
      {@const lockedIntegrated = isLockedIntegrated(item)}
      {@const reinstallPending = isReinstallPending(item)}
      {@const blocked = dependencyBlocked(item)}
      {@const checked = isCheckboxChecked(item)}
      {@const disabled = isCheckboxDisabled(item)}
      <div
        class={`flex items-start gap-3 rounded-xl border px-4 py-3 ${
          bundledLocked
            ? "border-white/5 bg-white/[0.01]"
            : lockedIntegrated
              ? "border-white/5 bg-white/[0.01]"
              : reinstallPending
                ? "border-amber-500/25 bg-amber-500/[0.04]"
                : checked
                  ? "border-indigo-500/20 bg-indigo-500/[0.04]"
                  : "border-white/10 bg-white/[0.02]"
        } ${blocked && !bundledLocked && !lockedIntegrated && !reinstallPending ? "opacity-50" : ""}`}
      >
        <input
          type="checkbox"
          class={`mt-1 shrink-0 ${disabled ? "cursor-not-allowed opacity-40" : "cursor-pointer"}`}
          checked={checked}
          disabled={disabled}
          onchange={() => toggle(item.id)}
        />
        <div class="min-w-0 flex-1">
          <div class="flex flex-wrap items-center gap-2">
            <span
              class={`text-sm font-medium ${disabled && (bundledLocked || lockedIntegrated || reinstallPending) ? "text-white/45" : "text-white/85"}`}
            >
              {item.label}
            </span>
            {#if bundledLocked}
              <span class="rounded-full bg-indigo-500/10 px-2 py-0.5 text-[10px] text-indigo-200/70">
                Terrain 内置
              </span>
            {:else if lockedIntegrated}
              <span class="rounded-full bg-white/5 px-2 py-0.5 text-[10px] text-white/35">
                已集成
              </span>
            {:else if reinstallPending}
              <span class="rounded-full bg-amber-500/15 px-2 py-0.5 text-[10px] text-amber-200/90">
                将重新安装
              </span>
            {/if}
            {#if item.optional}
              <span class="text-[10px] text-white/35">可选</span>
            {/if}
          </div>
          <p class={`mt-0.5 text-xs ${disabled && (bundledLocked || lockedIntegrated || reinstallPending) ? "text-white/30" : "text-white/45"}`}>
            {item.description}
          </p>
          <p class={`mt-1 text-[11px] ${disabled && (bundledLocked || lockedIntegrated || reinstallPending) ? "text-white/25" : "text-white/35"}`}>
            {item.detail}
          </p>
        </div>
        {#if canCancelReinstall(item)}
          <button
            type="button"
            class="shrink-0 rounded-lg border border-amber-500/25 px-2.5 py-1 text-[11px] text-amber-200/80 hover:border-amber-500/40 hover:bg-amber-500/10 hover:text-amber-100"
            onclick={() => cancelReinstall(item.id)}
          >
            取消重新安装
          </button>
        {:else if canMarkReinstall(item)}
          <button
            type="button"
            class="shrink-0 rounded-lg border border-white/10 px-2.5 py-1 text-[11px] text-white/55 hover:border-white/20 hover:bg-white/5 hover:text-white/75"
            onclick={() => markForReinstall(item.id)}
          >
            {item.bundled ? "重新部署内置工具" : "标记为重新安装"}
          </button>
        {/if}
      </div>
    {/snippet}

    <section>
      <h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-white/40">Skills</h3>
      <div class="grid gap-2">
        {#each skillItems as item}
          {@render itemRow(item)}
        {/each}
      </div>
    </section>

    <section>
      <h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-white/40">工具链</h3>
      <div class="grid gap-2">
        {#each toolItems as item}
          {@render itemRow(item)}
        {/each}
      </div>
    </section>

    <section>
      <h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-white/40">配置</h3>
      <div class="grid gap-2">
        {#each configItems as item}
          {@render itemRow(item)}
        {/each}
      </div>
    </section>

  {/if}
</div>

<EnvPlanPanel open={planHelpOpen} {plan} onclose={() => (planHelpOpen = false)} />
