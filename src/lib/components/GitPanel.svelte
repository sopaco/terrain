<script lang="ts">
  import { Check, ChevronDown, GitBranch, Loader2, TriangleAlert } from "@lucide/svelte";
  import {
    getGitBranches,
    getGitCommitDiff,
    getGitCommitFilesList,
    getGitDiffFile,
    getGitHistory,
    getGitStatus,
    gitCheckout,
    gitCommitFiles,
    gitFetch,
    gitMerge,
    gitPull,
    gitPush,
  } from "../api";
  import { tr } from "../i18n";
  import type {
    GitBranches,
    GitCommit,
    GitDiff,
    GitFileEntry,
    GitOpResult,
    GitStatus,
  } from "../types";

  interface Props {
    repoPath: string | null;
    onStatus?: (
      message: string,
      kind: "idle" | "loading" | "progress" | "success" | "error",
    ) => void;
  }

  let { repoPath, onStatus }: Props = $props();

  let status = $state<GitStatus | null>(null);
  let branches = $state<GitBranches | null>(null);
  let loading = $state(false);
  let busy = $state(false);

  const entries = $derived(status?.entries ?? []);
  const conflicted = $derived(entries.filter((e) => e.conflict));

  // Repo switch: drop panel-owned state (include set, selections).
  let lastRepo: string | null = null;
  let seen = new Set<string>();

  async function refresh() {
    const path = repoPath;
    if (!path) {
      status = null;
      branches = null;
      lastRepo = null;
      return;
    }
    if (path !== lastRepo) {
      lastRepo = path;
      seen = new Set();
      conflictFlag = false;
      dropdownOpen = false;
      mergeRowOpen = false;
    }
    loading = true;
    try {
      const [s, b] = await Promise.all([getGitStatus(path), getGitBranches(path)]);
      status = s;
      branches = b;
      if (!(s.merging || s.entries.some((e) => e.conflict))) conflictFlag = false;
      // Include set: prune vanished paths, auto-include newly appeared ones
      // (Desktop defaults new changes to included; explicit unchecks stick).
      const current = new Set(s.entries.map((e) => e.path));
      for (const p of [...include]) if (!current.has(p)) include.delete(p);
      for (const e of s.entries) {
        if (!seen.has(e.path)) {
          seen.add(e.path);
          include.add(e.path);
        }
      }
      // Diff selection: keep, or fall back to the first file when stale.
      if (selectedFile && !current.has(selectedFile)) selectedFile = null;
      if (!selectedFile) selectedFile = s.entries[0]?.path ?? null;
    } catch (e) {
      status = null;
      branches = null;
      onStatus?.(String(e), "error");
    } finally {
      loading = false;
    }
  }

  async function runGitOp(
    fn: () => Promise<GitOpResult>,
    opts: { announce?: boolean } = {},
  ): Promise<boolean> {
    if (!repoPath || busy) return false;
    busy = true;
    try {
      let res: GitOpResult;
      try {
        res = await fn();
      } catch (e) {
        onStatus?.(String(e), "error");
        return false;
      }
      if (!res.ok || res.output.includes("CONFLICT")) conflictFlag = true;
      if (!res.ok) {
        onStatus?.(res.output.trim() || "Git command failed", "error");
        return false;
      }
      if (opts.announce) onStatus?.(res.output.trim(), "success");
    } finally {
      busy = false;
      await refresh();
      if (view === "history") void loadHistory();
    }
    return true;
  }

  let conflictFlag = $state(false);
  let mergeBranch = $state("");
  let view = $state<"changes" | "history">("changes");
  let dropdownOpen = $state(false);
  let mergeRowOpen = $state(false);

  const ahead = $derived(status?.ahead ?? 0);
  const behind = $derived(status?.behind ?? 0);
  const mergeChoices = $derived.by(() => {
    const bs = branches;
    return bs ? bs.branches.filter((b) => b !== bs.current) : [];
  });

  async function doMerge() {
    const repo = repoPath;
    if (!repo || !mergeBranch) return;
    await runGitOp(() => gitMerge(repo, mergeBranch), { announce: true });
  }

  async function doCheckout(branch: string) {
    const repo = repoPath;
    if (!repo) return;
    dropdownOpen = false;
    mergeRowOpen = false;
    await runGitOp(() => gitCheckout(repo, branch), { announce: true });
  }

  async function doFetch() {
    const repo = repoPath;
    if (!repo) return;
    await runGitOp(() => gitFetch(repo), { announce: true });
  }

  async function doPush() {
    const repo = repoPath;
    if (!repo) return;
    await runGitOp(() => gitPush(repo), { announce: true });
  }

  async function doPull() {
    const repo = repoPath;
    if (!repo) return;
    await runGitOp(() => gitPull(repo), { announce: true });
  }

  // --- Changes view: Desktop-style include set (UI-owned, not the git index).
  let include = $state<Set<string>>(new Set());
  let includeAllEl = $state<HTMLInputElement | null>(null);
  let selectedFile = $state<string | null>(null);
  let diff = $state<GitDiff | null>(null);
  let diffLoading = $state(false);
  let summaryText = $state("");
  let descText = $state("");
  let diffSeq = 0; // plain counter: guards stale diff responses, never rendered

  const allIncluded = $derived(entries.length > 0 && entries.every((e) => include.has(e.path)));
  const someIncluded = $derived(entries.some((e) => include.has(e.path)));
  const inConflict = $derived(
    (status?.merging ?? false) || conflicted.length > 0 || conflictFlag,
  );

  function chipCode(entry: GitFileEntry): string {
    return entry.unstaged ?? entry.staged ?? "U";
  }

  function chipClass(entry: GitFileEntry): string {
    const code = chipCode(entry);
    if (code === "A") return "bg-tr-good-soft text-tr-on-good";
    if (code === "D") return "bg-tr-critical-soft text-tr-on-critical";
    if (code === "R") return "bg-tr-accent-soft text-tr-on-accent";
    if (code === "U") return "bg-tr-elevated text-tr-ink-3";
    return "bg-tr-watch-soft text-tr-on-watch"; // M
  }

  function toggleInclude(path: string) {
    if (include.has(path)) include.delete(path);
    else include.add(path);
  }

  function toggleAll() {
    if (allIncluded) include.clear();
    else for (const e of entries) include.add(e.path);
  }

  function lineClass(line: string): string {
    if (line.startsWith("+") && !line.startsWith("+++"))
      return "bg-[color:var(--color-tr-good-soft)] text-tr-ink-2";
    if (line.startsWith("-") && !line.startsWith("---"))
      return "bg-[color:var(--color-tr-critical-soft)] text-tr-ink-2";
    if (
      line.startsWith("@@") ||
      line.startsWith("diff --git") ||
      line.startsWith("index ") ||
      line.startsWith("\\")
    )
      return "text-tr-ink-4";
    return "";
  }

  async function commit() {
    const repo = repoPath;
    const summary = summaryText.trim();
    if (!repo || !summary || include.size === 0) return;
    const ok = await runGitOp(
      () => gitCommitFiles(repo, summary, descText.trim() || null, [...include]),
      { announce: true },
    );
    if (ok) {
      summaryText = "";
      descText = "";
      include.clear();
    }
  }

  // --- History view.
  let commits = $state<GitCommit[]>([]);
  let selectedHash = $state<string | null>(null);
  let commitFiles = $state<GitFileEntry[]>([]);
  let commitFile = $state<string | null>(null);
  let commitDiff = $state<GitDiff | null>(null);
  let historyLoading = $state(false);

  function relTime(secs: number): string {
    const delta = Date.now() / 1000 - secs;
    if (delta < 60) return "now";
    if (delta < 3600) return `${Math.floor(delta / 60)}m`;
    if (delta < 86_400) return `${Math.floor(delta / 3600)}h`;
    return `${Math.floor(delta / 86_400)}d`;
  }

  async function loadHistory() {
    const path = repoPath;
    if (!path) return;
    historyLoading = true;
    try {
      const list = await getGitHistory(path, 60);
      commits = list;
      if (selectedHash && !list.some((c) => c.hash === selectedHash)) {
        selectedHash = null;
        commitFiles = [];
        commitFile = null;
        commitDiff = null;
      }
      if (!selectedHash && list.length > 0) void selectCommit(list[0].hash);
    } catch (e) {
      onStatus?.(String(e), "error");
    } finally {
      historyLoading = false;
    }
  }

  async function selectCommit(hash: string) {
    selectedHash = hash;
    commitFile = null;
    commitDiff = null;
    const repo = repoPath;
    if (!repo) return;
    try {
      commitFiles = await getGitCommitFilesList(repo, hash);
      const first = commitFiles[0]?.path;
      if (first) void selectCommitFile(first);
    } catch (e) {
      commitFiles = [];
      onStatus?.(String(e), "error");
    }
  }

  async function selectCommitFile(path: string) {
    commitFile = path;
    const repo = repoPath;
    const hash = selectedHash;
    if (!repo || !hash) return;
    try {
      commitDiff = await getGitCommitDiff(repo, hash, path);
    } catch (e) {
      commitDiff = null;
      onStatus?.(String(e), "error");
    }
  }

  $effect(() => {
    if (includeAllEl) includeAllEl.indeterminate = someIncluded && !allIncluded;
  });

  $effect(() => {
    if (view === "history" && repoPath) void loadHistory();
  });

  $effect(() => {
    const repo = repoPath;
    const path = selectedFile;
    const entry = entries.find((e) => e.path === path);
    if (!repo || !path || !entry || entry.untracked) {
      diff = null;
      diffLoading = false;
      return;
    }
    const seq = ++diffSeq;
    diffLoading = true;
    getGitDiffFile(repo, path, false)
      .then((d) => {
        if (seq === diffSeq) diff = d;
      })
      .catch((e) => {
        if (seq === diffSeq) {
          diff = null;
          onStatus?.(String(e), "error");
        }
      })
      .finally(() => {
        if (seq === diffSeq) diffLoading = false;
      });
  });

  $effect(() => {
    void refresh();
  });

  $effect(() => {
    const choices = mergeChoices;
    if (choices.length > 0 && !choices.includes(mergeBranch)) {
      mergeBranch = choices[0] ?? "";
    }
  });
</script>

<div class="flex min-h-0 flex-1 flex-col">
  <header class="relative flex h-10 shrink-0 items-center gap-2 border-b border-tr-border px-3">
    <div class="relative">
      <button
        type="button"
        class="tr-press inline-flex max-w-52 items-center gap-1.5 rounded-lg border border-tr-border-strong px-2.5 py-1.5 text-xs font-medium text-tr-ink transition-colors hover:bg-tr-raised disabled:opacity-50"
        disabled={!status}
        onclick={() => (dropdownOpen = !dropdownOpen)}
      >
        <GitBranch size={14} class="shrink-0 text-tr-ink-3" aria-hidden="true" />
        <span class="min-w-0 truncate">{status?.branch ?? "—"}</span>
        <ChevronDown size={12} class="shrink-0 text-tr-ink-3" aria-hidden="true" />
      </button>
      {#if dropdownOpen}
        <button
          type="button"
          class="fixed inset-0 z-[9] cursor-default"
          aria-label="Close"
          onclick={() => (dropdownOpen = false)}
        ></button>
        <div
          class="absolute top-full left-0 z-10 mt-1 w-64 rounded-xl border border-tr-border-strong bg-tr-raised p-1 shadow-xl"
        >
          {#if entries.length > 0}
            <p class="px-2 py-1 text-[11px] text-tr-ink-3">{tr("git.switchHint")}</p>
          {/if}
          {#each branches?.branches ?? [] as b (b)}
            <button
              type="button"
              class="flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left text-xs text-tr-ink-2 transition-colors hover:bg-tr-elevated"
              onclick={() => void doCheckout(b)}
            >
              <span class="min-w-0 flex-1 truncate">{b}</span>
              {#if b === branches?.current}
                <Check size={14} class="shrink-0 text-tr-accent" aria-hidden="true" />
              {/if}
            </button>
          {/each}
          <div class="mt-1 border-t border-tr-border pt-1">
            {#if !mergeRowOpen}
              <button
                type="button"
                class="w-full rounded-lg px-2 py-1.5 text-left text-xs text-tr-ink-2 transition-colors hover:bg-tr-elevated"
                onclick={() => (mergeRowOpen = true)}
              >
                {tr("git.mergeInto", { branch: status?.branch ?? "" })}
              </button>
            {:else}
              <div class="flex items-center gap-1.5 p-1">
                <select
                  bind:value={mergeBranch}
                  aria-label={tr("git.mergeSelect")}
                  class="min-w-0 flex-1 rounded-lg border border-tr-border-strong bg-tr-elevated px-2 py-1 text-xs outline-none focus:border-tr-accent"
                >
                  {#each mergeChoices as b (b)}
                    <option value={b}>{b}</option>
                  {/each}
                </select>
                <button
                  type="button"
                  class="tr-press shrink-0 rounded-lg bg-tr-accent px-2.5 py-1 text-xs font-medium text-tr-on-accent transition-colors hover:bg-tr-accent-hover disabled:opacity-50"
                  disabled={busy || !mergeBranch}
                  onclick={() => void doMerge()}
                >
                  {tr("git.mergeBtn")}
                </button>
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <div class="flex items-center rounded-lg bg-tr-elevated/60 p-0.5" role="tablist" aria-label={tr("git.title")}>
      <button
        type="button"
        role="tab"
        aria-selected={view === "changes"}
        class="tr-press rounded-md px-2.5 py-1 text-xs font-medium transition-colors {view === "changes"
          ? "bg-tr-surface text-tr-ink"
          : "text-tr-ink-3 hover:text-tr-ink-2"}"
        onclick={() => (view = "changes")}
      >
        {tr("git.changes")}
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={view === "history"}
        class="tr-press rounded-md px-2.5 py-1 text-xs font-medium transition-colors {view === "history"
          ? "bg-tr-surface text-tr-ink"
          : "text-tr-ink-3 hover:text-tr-ink-2"}"
        onclick={() => (view = "history")}
      >
        {tr("git.history")}
      </button>
    </div>

    <div class="ml-auto flex items-center gap-2">
      <button
        type="button"
        class="tr-press inline-flex items-center gap-1.5 rounded-lg border border-tr-border-strong px-2.5 py-1.5 text-xs text-tr-ink-2 transition-colors hover:bg-tr-raised disabled:opacity-50"
        disabled={busy}
        onclick={() => void doFetch()}
      >
        {tr("git.fetchOrigin")}
        {#if ahead > 0}
          <span class="rounded-full bg-tr-elevated px-1.5 text-[10px] text-tr-ink-2">↑{ahead}</span>
        {/if}
        {#if behind > 0}
          <span class="rounded-full bg-tr-elevated px-1.5 text-[10px] text-tr-ink-2">↓{behind}</span>
        {/if}
      </button>
      <button
        type="button"
        class="tr-press inline-flex items-center rounded-lg border border-tr-border-strong px-2.5 py-1.5 text-xs text-tr-ink-2 transition-colors hover:bg-tr-raised disabled:opacity-50"
        disabled={busy}
        onclick={() => void doPush()}
      >
        {tr("git.push")}
      </button>
      <button
        type="button"
        class="tr-press inline-flex items-center rounded-lg border border-tr-border-strong px-2.5 py-1.5 text-xs text-tr-ink-2 transition-colors hover:bg-tr-raised disabled:opacity-50"
        disabled={busy}
        onclick={() => void doPull()}
      >
        {tr("git.pull")}
      </button>
    </div>
  </header>

  {#if !repoPath}
    <div class="flex flex-1 items-center justify-center">
      <p class="text-sm text-tr-ink-3">{tr("git.noProject")}</p>
    </div>
  {:else if loading && !status}
    <div class="flex flex-1 items-center justify-center gap-2 text-sm text-tr-ink-3" role="status" aria-busy="true">
      <Loader2 size={16} class="animate-spin" aria-hidden="true" />
      <span>{tr("git.busy")}</span>
    </div>
  {:else if status && view === "changes"}
    {#if inConflict}
      <div
        class="shrink-0 border-b border-tr-critical/30 bg-[color:var(--color-tr-critical-soft)] px-3 py-2"
        role="note"
      >
        <div class="flex items-start gap-2">
          <TriangleAlert size={14} class="mt-0.5 shrink-0 text-tr-critical" aria-hidden="true" />
          <div class="min-w-0 text-xs">
            <p class="font-medium text-tr-ink-2">{tr("git.conflictBanner")}</p>
            {#if conflicted.length > 0}
              <ul class="mt-1 space-y-0.5">
                {#each conflicted as ce (ce.path)}
                  <li class="truncate text-tr-critical" title={ce.path}>{ce.path}</li>
                {/each}
              </ul>
            {/if}
          </div>
        </div>
      </div>
    {/if}

    <div class="flex min-h-0 flex-1">
      <section class="flex w-[320px] shrink-0 flex-col border-r border-tr-border">
        <header class="flex shrink-0 items-center gap-2 border-b border-tr-border px-3 py-2">
          <input
            type="checkbox"
            class="size-3.5 shrink-0 cursor-pointer"
            bind:this={includeAllEl}
            checked={allIncluded}
            aria-label={tr("git.includeAll")}
            onchange={() => toggleAll()}
          />
          <span class="min-w-0 flex-1 truncate text-xs font-medium text-tr-ink-3">
            {tr("git.changesCount", { count: entries.length })}
          </span>
        </header>
        <div class="min-h-0 flex-1 overflow-y-auto">
          {#if entries.length === 0}
            <p class="px-3 py-6 text-center text-sm text-tr-ink-3">{tr("git.allClean")}</p>
          {:else}
            {#each entries as entry (entry.path)}
              <div
                class="flex items-center gap-2 px-3 py-1.5 text-xs transition-colors hover:bg-tr-elevated/60"
                class:bg-tr-accent-soft={selectedFile === entry.path}
              >
                <input
                  type="checkbox"
                  class="size-3.5 shrink-0 cursor-pointer"
                  checked={include.has(entry.path)}
                  onclick={(e) => e.stopPropagation()}
                  onchange={() => toggleInclude(entry.path)}
                />
                <button
                  type="button"
                  class="flex min-w-0 flex-1 items-center gap-2 text-left"
                  title={entry.path}
                  onclick={() => (selectedFile = entry.path)}
                >
                  <span class="shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold {chipClass(entry)}">
                    {chipCode(entry)}
                  </span>
                  <span class="min-w-0 flex-1 truncate text-tr-ink-2">{entry.path}</span>
                </button>
                {#if entry.conflict}
                  <span class="shrink-0 text-tr-critical" title={tr("git.conflictBanner")}>⚠</span>
                {/if}
              </div>
            {/each}
          {/if}
        </div>
        <footer class="shrink-0 border-t border-tr-border p-3">
          <input
            bind:value={summaryText}
            placeholder={tr("git.summaryPlaceholder")}
            maxlength="72"
            class="w-full rounded-lg border border-tr-border-strong bg-tr-elevated px-2.5 py-1.5 text-xs outline-none focus:border-tr-accent"
          />
          <textarea
            bind:value={descText}
            rows="3"
            placeholder={tr("git.descPlaceholder")}
            class="mt-2 w-full resize-none rounded-lg border border-tr-border-strong bg-tr-elevated px-2.5 py-1.5 text-xs outline-none focus:border-tr-accent"
          ></textarea>
          <button
            type="button"
            class="tr-press mt-2 inline-flex w-full items-center justify-center gap-1.5 rounded-lg bg-tr-accent px-3 py-1.5 text-xs font-medium text-tr-on-accent transition-colors hover:bg-tr-accent-hover disabled:opacity-50"
            disabled={busy || !summaryText.trim() || include.size === 0}
            onclick={() => void commit()}
          >
            {tr("git.commitTo", { branch: status.branch })}
          </button>
        </footer>
      </section>

      <section class="flex min-w-0 flex-1 flex-col">
        {#if selectedFile}
          {@const selEntry = entries.find((e) => e.path === selectedFile)}
          <header class="flex h-9 shrink-0 items-center gap-2 border-b border-tr-border px-3 text-xs">
            {#if selEntry}
              <span class="shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold {chipClass(selEntry)}">
                {chipCode(selEntry)}
              </span>
            {/if}
            <span class="min-w-0 truncate text-tr-ink-2">{selectedFile}</span>
          </header>
          {#if selEntry?.untracked}
            <p class="p-4 text-sm text-tr-ink-3">{tr("git.untrackedNoDiff")}</p>
          {:else if diffLoading}
            <div class="flex flex-1 items-center justify-center" role="status" aria-busy="true">
              <Loader2 size={16} class="animate-spin text-tr-ink-3" aria-hidden="true" />
            </div>
          {:else if diff && diff.text}
            {@render diffView(diff)}
          {:else}
            <p class="p-4 text-sm text-tr-ink-3">{tr("git.noDiff")}</p>
          {/if}
        {:else}
          <p class="p-4 text-sm text-tr-ink-3">{tr("git.allClean")}</p>
        {/if}
      </section>
    </div>
  {:else if status}
    <div class="flex min-h-0 flex-1">
      <section class="w-[320px] shrink-0 overflow-y-auto border-r border-tr-border">
        {#if historyLoading && commits.length === 0}
          <div class="flex items-center justify-center py-6" role="status" aria-busy="true">
            <Loader2 size={16} class="animate-spin text-tr-ink-3" aria-hidden="true" />
          </div>
        {:else}
          {#each commits as c (c.hash)}
            <button
              type="button"
              class="block w-full border-b border-tr-border/60 px-3 py-2 text-left transition-colors hover:bg-tr-elevated/60"
              class:bg-tr-accent-soft={selectedHash === c.hash}
              onclick={() => void selectCommit(c.hash)}
            >
              <p class="truncate text-xs text-tr-ink">{c.summary}</p>
              <p class="mt-0.5 flex items-center gap-1.5 text-[11px] text-tr-ink-3">
                <span class="min-w-0 truncate">
                  <span class="sr-only">{tr("git.author")}: </span>{c.author}
                </span>
                <span aria-hidden="true">·</span>
                <span class="shrink-0 whitespace-nowrap">{relTime(Number(c.time_secs))}</span>
                <span class="ml-auto shrink-0 font-mono text-tr-ink-4">{c.short}</span>
              </p>
            </button>
          {/each}
        {/if}
      </section>

      <section class="flex min-w-0 flex-1 flex-col">
        {#if selectedHash}
          <div class="min-h-0 flex-1 overflow-y-auto">
            <p class="px-3 py-2 text-[11px] font-medium text-tr-ink-3">{tr("git.filesChanged")}</p>
            {#each commitFiles as entry (entry.path)}
              <button
                type="button"
                class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs transition-colors hover:bg-tr-elevated/60"
                class:bg-tr-accent-soft={commitFile === entry.path}
                onclick={() => void selectCommitFile(entry.path)}
              >
                <span class="shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold {chipClass(entry)}">
                  {chipCode(entry)}
                </span>
                <span class="min-w-0 flex-1 truncate text-tr-ink-2" title={entry.path}>{entry.path}</span>
              </button>
            {/each}
            {#if commitFile}
              <div class="border-t border-tr-border">
                {#if commitDiff && commitDiff.text}
                  {@render diffView(commitDiff)}
                {:else}
                  <p class="p-4 text-sm text-tr-ink-3">{tr("git.noDiff")}</p>
                {/if}
              </div>
            {/if}
          </div>
        {/if}
      </section>
    </div>
  {/if}

  {#snippet diffView(d: GitDiff)}
    <div class="min-h-0 flex-1 overflow-auto">
      <pre class="min-w-max px-3 py-2 font-mono text-[11px] leading-4">{#each d.text.split("\n") as line, i (i)}<div class={lineClass(line)}>{line || " "}</div>{/each}</pre>
      {#if d.truncated}
        <p class="px-3 pb-2 text-[11px] text-tr-ink-3">{tr("git.truncated")}</p>
      {/if}
    </div>
  {/snippet}
</div>