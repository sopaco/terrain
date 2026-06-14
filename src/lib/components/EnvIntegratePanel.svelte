<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { getEnvStatus, planEnvIntegration, runEnvIntegration } from "../api";
  import type { EnvIntegrationStatus, EnvPlan, EnvStatus } from "../types";

  interface Props {
    repoPath: string | null;
    onStatus?: (message: string, kind: "idle" | "loading" | "progress" | "success" | "error") => void;
    onIntegrated?: () => void;
  }

  let { repoPath, onStatus, onIntegrated }: Props = $props();

  let status = $state<EnvStatus | null>(null);
  let plan = $state<EnvPlan | null>(null);
  let loading = $state(false);
  let applying = $state(false);
  let progressMessage = $state<string | null>(null);
  /** IDs selected for install / reinstall (excludes locked integrated items). */
  let selected = $state<Set<string>>(new Set());
  /** Integrated items explicitly marked for reinstall. */
  let reinstallMarked = $state<Set<string>>(new Set());

  const skillItems = $derived(status?.items.filter((i) => i.kind === "skill") ?? []);
  const toolItems = $derived(status?.items.filter((i) => i.kind === "tool") ?? []);
  const configItems = $derived(
    status?.items.filter((i) => i.kind === "agents_md" || i.kind === "gitignore") ?? [],
  );

  const applyCount = $derived(selected.size);

  function isLockedIntegrated(item: EnvIntegrationStatus): boolean {
    return item.integrated && !reinstallMarked.has(item.id);
  }

  function dependencySatisfied(depId: string): boolean {
    const dep = status?.items.find((i) => i.id === depId);
    if (!dep) return false;
    if (dep.integrated && !reinstallMarked.has(depId)) return true;
    return selected.has(depId);
  }

  function dependencyBlocked(item: EnvIntegrationStatus): boolean {
    if (isLockedIntegrated(item)) return false;
    return item.depends_on.some((d) => !dependencySatisfied(d));
  }

  async function loadStatus() {
    if (!repoPath) {
      status = null;
      plan = null;
      return;
    }
    loading = true;
    try {
      status = await getEnvStatus(repoPath);
      selected = new Set(status.items.filter((i) => !i.integrated).map((i) => i.id));
      reinstallMarked = new Set();
      await refreshPlan(selected);
    } catch (e) {
      status = null;
      onStatus?.(String(e), "error");
    } finally {
      loading = false;
    }
  }

  async function refreshPlan(ids: Set<string>) {
    if (!repoPath) return;
    try {
      plan = await planEnvIntegration(repoPath, [...ids]);
    } catch {
      plan = null;
    }
  }

  function toggle(id: string) {
    const item = status?.items.find((i) => i.id === id);
    if (!item || isLockedIntegrated(item) || dependencyBlocked(item)) return;

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
    void refreshPlan(next);
  }

  function markForReinstall(id: string) {
    const item = status?.items.find((i) => i.id === id);
    if (!item?.integrated || reinstallMarked.has(id)) return;

    const next = new Set(selected);
    const nextReinstall = new Set(reinstallMarked);
    next.add(id);
    nextReinstall.add(id);
    selected = next;
    reinstallMarked = nextReinstall;
    void refreshPlan(next);
  }

  function selectAllPending() {
    if (!status) return;
    selected = new Set(status.items.filter((i) => !i.integrated).map((i) => i.id));
    reinstallMarked = new Set();
    void refreshPlan(selected);
  }

  function selectNone() {
    selected = new Set();
    reinstallMarked = new Set();
    void refreshPlan(selected);
  }

  async function apply() {
    if (!repoPath || selected.size === 0) return;
    applying = true;
    progressMessage = "准备集成…";
    onStatus?.("正在集成 AI 工程环境…", "progress");
    try {
      const result = await runEnvIntegration(repoPath, [...selected]);
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
    <h2 class="text-lg font-semibold text-white/90">AI 工程环境集成</h2>
    <p class="mt-1 text-sm text-white/50">
      为 Coding Agent（Claude Code、Cursor 等）配置 Skills、工具链与 AGENTS.md。
      知识资产使用仓库内 <code class="text-white/70">.mind-mesh/</code>，由 MindMesh 生成。
    </p>
    {#if status}
      <p class="mt-2 text-sm text-indigo-200/80">{status.summary}</p>
    {/if}
  </header>

  {#if !repoPath}
    <p class="text-sm text-white/45">请先选择已索引的项目。</p>
  {:else if loading}
    <p class="text-sm text-white/45">正在检测工程环境…</p>
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
        清空选择
      </button>
      <button
        type="button"
        class="rounded-lg bg-indigo-600 px-4 py-1.5 text-xs font-medium hover:bg-indigo-500 disabled:opacity-50"
        disabled={applying || applyCount === 0}
        onclick={apply}
      >
        {applying ? "集成中…" : `集成所选 (${applyCount})`}
      </button>
      {#if progressMessage}
        <span class="text-xs text-white/45">{progressMessage}</span>
      {/if}
    </div>

    {#snippet itemRow(item: EnvIntegrationStatus)}
      {@const locked = isLockedIntegrated(item)}
      {@const blocked = dependencyBlocked(item)}
      {@const checked = locked || selected.has(item.id)}
      <div
        class={`flex items-start gap-3 rounded-xl border px-4 py-3 ${
          locked
            ? "border-white/5 bg-white/[0.01]"
            : item.integrated && reinstallMarked.has(item.id)
              ? "border-amber-500/25 bg-amber-500/[0.04]"
              : checked
                ? "border-indigo-500/20 bg-indigo-500/[0.04]"
                : "border-white/10 bg-white/[0.02]"
        } ${blocked && !locked ? "opacity-50" : ""}`}
      >
        <input
          type="checkbox"
          class={`mt-1 shrink-0 ${locked ? "cursor-not-allowed opacity-40" : "cursor-pointer"}`}
          checked={checked}
          disabled={locked || blocked}
          onchange={() => toggle(item.id)}
        />
        <div class="min-w-0 flex-1">
          <div class="flex flex-wrap items-center gap-2">
            <span
              class={`text-sm font-medium ${locked ? "text-white/45" : "text-white/85"}`}
            >
              {item.label}
            </span>
            {#if locked}
              <span class="rounded-full bg-white/5 px-2 py-0.5 text-[10px] text-white/35">
                已集成
              </span>
            {:else if item.integrated && reinstallMarked.has(item.id)}
              <span class="rounded-full bg-amber-500/15 px-2 py-0.5 text-[10px] text-amber-200/90">
                将重新安装
              </span>
            {/if}
            {#if item.optional}
              <span class="text-[10px] text-white/35">可选</span>
            {/if}
          </div>
          <p class={`mt-0.5 text-xs ${locked ? "text-white/30" : "text-white/45"}`}>
            {item.description}
          </p>
          <p class={`mt-1 text-[11px] ${locked ? "text-white/25" : "text-white/35"}`}>
            {item.detail}
          </p>
        </div>
        {#if locked}
          <button
            type="button"
            class="shrink-0 rounded-lg border border-white/10 px-2.5 py-1 text-[11px] text-white/55 hover:border-white/20 hover:bg-white/5 hover:text-white/75"
            onclick={() => markForReinstall(item.id)}
          >
            标记为重新安装
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

    {#if plan && plan.steps.length > 0}
      <section class="rounded-xl border border-white/10 bg-white/[0.02] p-4">
        <h3 class="text-xs font-semibold uppercase tracking-wider text-white/40">执行计划</h3>
        <ul class="mt-2 space-y-1 text-xs text-white/55">
          {#each plan.steps as step}
            <li>· {step.action}</li>
          {/each}
        </ul>
        {#if plan.skipped.length > 0}
          <p class="mt-2 text-[11px] text-white/35">跳过：{plan.skipped.join("；")}</p>
        {/if}
      </section>
    {/if}
  {/if}
</div>
