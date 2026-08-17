<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { getEnvStatus, planEnvIntegration, runEnvIntegration } from "../api";
  import type { EnvIntegrationStatus, EnvPlan, EnvStatus } from "../types";
  import { tr } from "../i18n";
  import EnvPlanPanel from "./EnvPlanPanel.svelte";
  import HelpButton from "./icons/HelpButton.svelte";

  interface Props {
    repoPath: string | null;
    onStatus?: (message: string, kind: "idle" | "loading" | "progress" | "success" | "error") => void;
    onIntegrated?: () => void;
  }

  let { repoPath, onStatus, onIntegrated }: Props = $props();

  let status = $state<EnvStatus | null>(null);
  let plan = $state<EnvPlan | null>(null);
  let loading = $state(false);
  let loadingHint = $state(tr("env.loading.readingList"));
  let applying = $state(false);
  let progressMessage = $state<string | null>(null);
  let selected = $state<Set<string>>(new Set());
  let reinstallMarked = $state<Set<string>>(new Set());
  let planHelpOpen = $state(false);

  const skillItems = $derived(status?.items.filter((i) => i.kind === "skill") ?? []);
  const toolItems = $derived(status?.items.filter((i) => i.kind === "tool") ?? []);
  const configItems = $derived(
    status?.items.filter(
      (i) => i.kind === "agents_md" || i.kind === "gitignore" || i.kind === "terrain_ignore",
    ) ?? [],
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
    loadingHint = tr("env.loading.detecting");
    try {
      status = await getEnvStatus(repoPath);
      loadingHint = tr("env.loading.planning");
      selected = defaultSelectedIds(status.items);
      reinstallMarked = new Set();
      await refreshPlan(selected);
    } catch (e) {
      status = null;
      onStatus?.(String(e), "error");
    } finally {
      loading = false;
      loadingHint = tr("env.loading.readingList");
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
    progressMessage = tr("env.apply.preparing");
    onStatus?.(tr("env.apply.progress"), "progress");
    try {
      const result = await runEnvIntegration(repoPath, [...selected], [...reinstallMarked]);
      if (result.errors.length > 0) {
        onStatus?.(tr("env.apply.partialFailed", { errors: result.errors.join("; ") }), "error");
      } else {
        onStatus?.(tr("env.apply.success", { count: result.applied.length }), "success");
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
    <h2 class="text-lg font-semibold text-tr-ink">{tr("terms.agentEnv")}</h2>
    <p class="mt-1 text-sm text-tr-ink-3">
      {tr("env.descLine1")}
      {tr("env.descAssetsPre")}<code class="text-tr-ink-2">.terrain/</code>{tr("env.descAssetsPost")}
    </p>
    {#if status}
      <p class="mt-2 text-sm text-tr-accent">{status.summary}</p>
    {/if}
  </header>

  {#if !repoPath}
    <p class="text-sm text-tr-ink-3">{tr("env.noProject")}</p>
  {:else if loading}
    <div
      class="rounded-xl border border-tr-accent-soft bg-tr-accent-soft px-4 py-4"
      role="status"
      aria-live="polite"
      aria-busy="true"
    >
      <div class="flex items-start gap-3">
        <div
          class="mt-0.5 h-5 w-5 shrink-0 animate-spin rounded-full border-2 border-tr-accent-soft-strong border-t-tr-accent"
        ></div>
        <div class="min-w-0">
          <p class="text-sm font-medium text-tr-ink-2">{tr("env.detectingTitle", { env: tr("terms.agentEnv") })}</p>
          <p class="mt-1 text-xs text-tr-ink-3">{loadingHint}</p>
          <p class="mt-2 text-[11px] text-tr-ink-3">
            {tr("env.loading.scanHint")}
          </p>
        </div>
      </div>
    </div>

    {#snippet skeletonSection(title: string, rows: number)}
      <section>
        <div class="mb-2 h-3 w-20 animate-pulse rounded bg-tr-elevated"></div>
        <div class="grid gap-2">
          {#each Array.from({ length: rows }, (_, i) => i) as i (i)}
            <div
              class="flex items-start gap-3 rounded-xl border border-tr-border bg-tr-elevated px-4 py-3"
            >
              <div class="mt-1 h-4 w-4 shrink-0 animate-pulse rounded bg-tr-elevated"></div>
              <div class="min-w-0 flex-1 space-y-2">
                <div class="h-3.5 w-2/5 max-w-[12rem] animate-pulse rounded bg-tr-elevated"></div>
                <div class="h-2.5 w-full animate-pulse rounded bg-tr-raised"></div>
                <div class="h-2.5 w-4/5 max-w-[20rem] animate-pulse rounded bg-tr-elevated"></div>
              </div>
            </div>
          {/each}
        </div>
      </section>
    {/snippet}

    <div class="mt-4 space-y-6 opacity-80">
      {@render skeletonSection("Skills", 4)}
      {@render skeletonSection(tr("env.section.tools"), 3)}
      {@render skeletonSection(tr("env.section.config"), 3)}
    </div>
  {:else if status}
    <div class="flex flex-wrap items-center gap-2">
      <button
        type="button"
        class="tr-press rounded-lg border border-tr-border-strong px-3 py-1.5 text-xs transition-colors hover:bg-tr-elevated"
        onclick={selectAllPending}
      >
        {tr("env.selectAllPending")}
      </button>
      <button
        type="button"
        class="tr-press rounded-lg border border-tr-border-strong px-3 py-1.5 text-xs transition-colors hover:bg-tr-elevated"
        onclick={selectNone}
      >
        {tr("env.clearSelection")}
      </button>
      <div class="flex items-center gap-1.5">
        <button
          type="button"
          class="tr-press rounded-lg bg-tr-accent px-4 py-1.5 text-xs font-medium transition-colors hover:bg-tr-accent-hover disabled:opacity-50"
          disabled={applying || applyCount === 0}
          onclick={apply}
        >
          {applying ? tr("env.applying") : tr("env.applySelected", { count: applyCount })}
        </button>
        {#if canApply}
          <HelpButton
            onclick={() => (planHelpOpen = true)}
            title={tr("env.planHelpTitle")}
            ariaLabel={tr("env.planHelpAria")}
          />
        {/if}
      </div>
      {#if progressMessage}
        <span class="text-xs text-tr-ink-3">{progressMessage}</span>
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
            ? "border-tr-border bg-tr-elevated"
            : lockedIntegrated
              ? "border-tr-border bg-tr-elevated"
              : reinstallPending
                ? "border-tr-watch/30 bg-tr-watch-soft"
                : checked
                  ? "border-tr-accent-soft-strong bg-tr-accent-soft"
                  : "border-tr-border-strong bg-tr-elevated"
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
              class={`text-sm font-medium ${disabled && (bundledLocked || lockedIntegrated || reinstallPending) ? "text-tr-ink-3" : "text-tr-ink-2"}`}
            >
              {item.label}
            </span>
            {#if bundledLocked}
              <span class="rounded-full bg-tr-accent-soft px-2 py-0.5 text-[10px] text-tr-accent">
                {tr("env.badge.bundled")}
              </span>
            {:else if lockedIntegrated}
              <span class="rounded-full bg-tr-elevated px-2 py-0.5 text-[10px] text-tr-ink-3">
                {tr("env.badge.integrated")}
              </span>
            {:else if reinstallPending}
              <span class="rounded-full bg-tr-watch-soft px-2 py-0.5 text-[10px] text-tr-watch">
                {tr("env.badge.reinstall")}
              </span>
            {/if}
            {#if item.optional}
              <span class="text-[10px] text-tr-ink-3">{tr("common.optional")}</span>
            {/if}
          </div>
          <p class={`mt-0.5 text-xs ${disabled && (bundledLocked || lockedIntegrated || reinstallPending) ? "text-tr-ink-3" : "text-tr-ink-3"}`}>
            {item.description}
          </p>
          <p class={`mt-1 text-[11px] ${disabled && (bundledLocked || lockedIntegrated || reinstallPending) ? "text-tr-ink-4" : "text-tr-ink-3"}`}>
            {item.detail}
          </p>
        </div>
        {#if canCancelReinstall(item)}
          <button
            type="button"
            class="tr-press shrink-0 rounded-lg border border-tr-watch/30 px-2.5 py-1 text-[11px] text-tr-watch transition-colors hover:border-tr-watch/40 hover:bg-tr-watch-soft hover:text-tr-watch"
            onclick={() => cancelReinstall(item.id)}
          >
            {tr("env.cancelReinstall")}
          </button>
        {:else if canMarkReinstall(item)}
          <button
            type="button"
            class="tr-press shrink-0 rounded-lg border border-tr-border-strong px-2.5 py-1 text-[11px] text-tr-ink-2 transition-colors hover:border-tr-border-strong hover:bg-tr-elevated hover:text-tr-ink-2"
            onclick={() => markForReinstall(item.id)}
          >
            {item.bundled ? tr("env.redeployBundled") : tr("env.markReinstall")}
          </button>
        {/if}
      </div>
    {/snippet}

    <section>
      <h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-tr-ink-3">Skills</h3>
      <div class="grid gap-2">
        {#each skillItems as item}
          {@render itemRow(item)}
        {/each}
      </div>
    </section>

    <section>
      <h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-tr-ink-3">{tr("env.section.tools")}</h3>
      <div class="grid gap-2">
        {#each toolItems as item}
          {@render itemRow(item)}
        {/each}
      </div>
    </section>

    <section>
      <h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-tr-ink-3">{tr("env.section.config")}</h3>
      <div class="grid gap-2">
        {#each configItems as item}
          {@render itemRow(item)}
        {/each}
      </div>
    </section>

  {/if}
</div>

<EnvPlanPanel open={planHelpOpen} {plan} onclose={() => (planHelpOpen = false)} />
