<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";
  import AskBar from "./lib/components/AskBar.svelte";
  import DeepWikiPanel from "./lib/components/DeepWikiPanel.svelte";
  import HumanDocTree from "./lib/components/HumanDocTree.svelte";
  import EnvIntegratePanel from "./lib/components/EnvIntegratePanel.svelte";
  import MainNavTabs from "./lib/components/MainNavTabs.svelte";
  import KnowledgeArticle from "./lib/components/KnowledgeArticle.svelte";
  import SourceDrawer from "./lib/components/SourceDrawer.svelte";
  import ProjectOverviewPanel from "./lib/components/ProjectOverviewPanel.svelte";
  import ProjectSelector from "./lib/components/ProjectSelector.svelte";
  import SddWorkflowPanel from "./lib/components/SddWorkflowPanel.svelte";
  import HelpPanel from "./lib/components/HelpPanel.svelte";
  import SettingsPanel from "./lib/components/SettingsPanel.svelte";
  import StatusBanner from "./lib/components/StatusBanner.svelte";
  import TaskProgressBar from "./lib/components/TaskProgressBar.svelte";
  import type { StatusKind } from "./lib/components/StatusBanner.svelte";
  import {
    checkLlm,
    checkAcp,
    computeFreshness,
    getKnowledgeRoot,
    getProjectOverview,
    initializeProject,
    listHumanDocs,
    listProjects,
    readProjectFreshnessCached,
    removeProject,
    listStaleProjects,
    openRepoFolder,
    packAgentAssets,
    readDocument,
    runAgentContextGeneration,
    runLithoGeneration,
    runQuickRefresh,
    searchKnowledge,
  } from "./lib/api";
  import { mergeFreshnessIntoOverview } from "./lib/mergeFreshness";
  import { generateLabel, TERMS } from "./lib/terminology";
  import { citationToSourceSlice } from "./lib/resolveSource";
  import type {
    AppTab,
    ChatMessage,
    HumanDocEntry,
    KnowledgeDoc,
    LlmStatus,
    ProjectOverview,
    ProjectSummary,
    SearchHit,
    SourceCitation,
    SourceSlice,
    StaleProjectSummary,
  } from "./lib/types";

  type ProjectTaskState = {
    repackBusy: boolean;
    lithoBusy: boolean;
    lithoProgress: string;
  };

  let projects = $state<ProjectSummary[]>([]);
  let staleProjects = $state<StaleProjectSummary[]>([]);
  let knowledgeRoot = $state("");
  let acpOk = $state(false);
  let llmStatus = $state<LlmStatus | null>(null);

  let query = $state("");
  let hits = $state<SearchHit[]>([]);
  let activeDoc = $state<KnowledgeDoc | null>(null);
  let selectedProject = $state<string | null>(null);
  let selectedRepoPath = $state<string | null>(null);
  let humanDocs = $state<HumanDocEntry[]>([]);
  let activeHumanPath = $state<string | null>(null);
  let humanDocsLoading = $state(false);

  let projectTasks = $state<Record<string, ProjectTaskState>>({});
  let chatSessions = $state<Record<string, ChatMessage[]>>({});
  let deepWikiSources = $state<Record<string, SourceSlice | null>>({});
  let knowledgeSources = $state<Record<string, SourceSlice | null>>({});

  let projectPickerOpen = $state(false);
  let settingsOpen = $state(false);
  let helpOpen = $state(false);
  let deepWikiOpen = $state(false);
  let deepWikiInitialQuestion = $state<string | null>(null);
  let activeTab = $state<AppTab>("overview");
  let projectOverview = $state<ProjectOverview | null>(null);
  let overviewLoading = $state(false);
  let freshnessLoading = $state(false);
  let agentContextBusy = $state(false);
  let quickRefreshBusy = $state(false);
  let initBusy = $state(false);
  let initProgress = $state<string | null>(null);

  let docLoading = $state(false);
  let statusMessage = $state("就绪");
  let statusKind = $state<StatusKind>("idle");
  let statusDetail = $state<string | null>(null);

  const selectedProjectMeta = $derived(
    projects.find((p) => p.slug === selectedProject) ?? null,
  );
  const currentTask = $derived(
    selectedProject
      ? (projectTasks[selectedProject] ?? { repackBusy: false, lithoBusy: false, lithoProgress: "" })
      : { repackBusy: false, lithoBusy: false, lithoProgress: "" },
  );
  const currentMessages = $derived(
    selectedProject ? (chatSessions[selectedProject] ?? []) : [],
  );
  const currentDeepWikiSource = $derived(
    selectedProject ? (deepWikiSources[selectedProject] ?? null) : null,
  );
  const knowledgeSourceSlice = $derived(
    selectedProject ? (knowledgeSources[selectedProject] ?? null) : null,
  );
  const repackBusy = $derived(currentTask.repackBusy);
  const lithoBusy = $derived(currentTask.lithoBusy);
  const lithoProgress = $derived(currentTask.lithoProgress);

  function setStatus(message: string, kind: StatusKind = "idle", detail: string | null = null) {
    statusMessage = message;
    statusKind = kind;
    statusDetail = detail;
  }

  function setProjectTask(slug: string, patch: Partial<ProjectTaskState>) {
    const prev = projectTasks[slug] ?? { repackBusy: false, lithoBusy: false, lithoProgress: "" };
    projectTasks = { ...projectTasks, [slug]: { ...prev, ...patch } };
  }

  function updateChat(
    slug: string,
    update: ChatMessage[] | ((prev: ChatMessage[]) => ChatMessage[]),
  ) {
    const prev = chatSessions[slug] ?? [];
    const next = typeof update === "function" ? update(prev) : update;
    chatSessions = { ...chatSessions, [slug]: next };
  }

  function setDeepWikiSource(slug: string, slice: SourceSlice | null) {
    deepWikiSources = { ...deepWikiSources, [slug]: slice };
  }

  function setKnowledgeSource(slug: string, slice: SourceSlice | null) {
    knowledgeSources = { ...knowledgeSources, [slug]: slice };
  }

  async function refreshKnowledgeRoot(slug?: string | null) {
    const target = slug ?? selectedProject;
    if (!target) {
      knowledgeRoot = "";
      return;
    }
    try {
      knowledgeRoot = await getKnowledgeRoot(target);
    } catch {
      knowledgeRoot = "";
    }
  }

  async function refresh() {
    setStatus("正在刷新项目列表…", "loading");
    try {
      [projects, staleProjects, acpOk, llmStatus] = await Promise.all([
        listProjects(),
        listStaleProjects(),
        checkAcp(),
        checkLlm(),
      ]);
      if (!selectedProject && projects.length > 0) {
        await selectProject(projects[0]);
      } else {
        await refreshKnowledgeRoot();
      }
      setStatus(`已索引 ${projects.length} 个项目`, "success");
    } catch (e) {
      setStatus(String(e), "error");
    }
  }

  async function loadProjectOverviewFreshness(slug: string, repoPath: string | null | undefined) {
    if (!repoPath) {
      freshnessLoading = false;
      return;
    }
    const requestSlug = slug;
    freshnessLoading = true;
    try {
      const cached = await readProjectFreshnessCached(slug);
      if (cached && selectedProject === requestSlug && projectOverview) {
        projectOverview = mergeFreshnessIntoOverview(projectOverview, cached);
      }

      const freshness = await computeFreshness(slug, repoPath);
      if (selectedProject === requestSlug && projectOverview) {
        projectOverview = mergeFreshnessIntoOverview(projectOverview, freshness);
      }
    } catch {
      /* keep cached or empty freshness */
    } finally {
      if (selectedProject === requestSlug) {
        freshnessLoading = false;
      }
    }
  }

  async function loadProjectOverview(slug: string, opts?: { skipFreshness?: boolean }) {
    overviewLoading = true;
    if (!opts?.skipFreshness) {
      freshnessLoading = false;
    }
    try {
      projectOverview = await getProjectOverview(slug);
      if (projectOverview?.repo_path && !opts?.skipFreshness) {
        void loadProjectOverviewFreshness(slug, projectOverview.repo_path);
      }
    } catch {
      projectOverview = null;
    } finally {
      overviewLoading = false;
    }
  }

  async function loadHumanDocs(slug: string) {
    humanDocsLoading = true;
    try {
      humanDocs = await listHumanDocs(slug);
    } catch {
      humanDocs = [];
    } finally {
      humanDocsLoading = false;
    }
  }

  async function addProject() {
    projectPickerOpen = false;
    if (initBusy) return;
    let picked: string | null;
    try {
      picked = await open({ directory: true, multiple: false });
    } catch (e) {
      setStatus(`选择文件夹失败：${e}`, "error");
      return;
    }
    if (!picked || Array.isArray(picked)) return;
    await triggerProjectInitialization(picked);
  }

  async function removeProjectFromList(project: ProjectSummary) {
    projectPickerOpen = false;
    try {
      await removeProject(project.slug);
      if (selectedProject === project.slug) {
        selectedProject = null;
        selectedRepoPath = null;
        projectOverview = null;
        activeDoc = null;
        activeHumanPath = null;
        humanDocs = [];
        hits = [];
      }
      await refresh();
      setStatus(`已从列表移除：${project.name}`, "success");
    } catch (e) {
      setStatus(String(e), "error");
    }
  }

  async function selectProject(project: ProjectSummary) {
    projectPickerOpen = false;
    if (selectedProject !== project.slug) {
      activeDoc = null;
      activeHumanPath = null;
      hits = [];
      deepWikiInitialQuestion = null;
    }
    selectedProject = project.slug;
    selectedRepoPath = project.repo_path ?? null;
    if (!selectedRepoPath) {
      try {
        const doc = await readDocument(project.path);
        const source = doc.frontmatter.source;
        selectedRepoPath = typeof source === "string" ? source : null;
      } catch {
        selectedRepoPath = null;
      }
    }
    setStatus(`项目：${project.name}`, "idle", project.slug);
    await Promise.all([
      loadHumanDocs(project.slug),
      loadProjectOverview(project.slug),
      refreshKnowledgeRoot(project.slug),
    ]);
  }

  async function openFolderPath(path: string) {
    try {
      await openRepoFolder(path);
    } catch (e) {
      setStatus(`Failed to open folder: ${e}`, "error");
    }
  }

  async function openProjectFolder(project: ProjectSummary) {
    const repo = project.repo_path ?? (project.slug === selectedProject ? selectedRepoPath : null);
    if (!repo) {
      setStatus(`No repository path for ${project.name}`, "error");
      return;
    }
    await openFolderPath(repo);
  }

  function parseProgressLabel(label: string): { stage: string | null; message: string } {
    const match = label.match(/^\[([^\]]+)\]\s*(.*)$/);
    if (!match) return { stage: null, message: label };
    return { stage: match[1], message: match[2] || label };
  }

  const lithoProgressParts = $derived(parseProgressLabel(lithoProgress));

  const showTaskProgressBar = $derived(
    Boolean(selectedProject && (lithoProgress || (initBusy && initProgress))),
  );

  const showStatusBar = $derived(
    !showTaskProgressBar && (statusKind !== "idle" || statusMessage !== "就绪"),
  );

  function openArchitectureDoc() {
    if (!projectOverview?.agent_context.path || !selectedProject) return;
    void (async () => {
      activeTab = "knowledge";
      docLoading = true;
      try {
        await loadHumanDocs(selectedProject);
        activeDoc = await readDocument(projectOverview.agent_context.path);
        activeHumanPath = projectOverview.agent_context.path;
      } catch (e) {
        setStatus(String(e), "error");
      } finally {
        docLoading = false;
      }
    })();
  }

  async function triggerProjectInitialization(repoPath: string, slug?: string) {
    if (initBusy) return;
    initBusy = true;
    initProgress = "正在扫描仓库…";
    const targetSlug = slug ?? null;
    if (targetSlug) {
      selectedProject = targetSlug;
      selectedRepoPath = repoPath;
      setProjectTask(targetSlug, { repackBusy: true, lithoBusy: false, lithoProgress: "" });
    }
    try {
      const result = await initializeProject(repoPath, slug);
      selectedProject = result.project_slug;
      selectedRepoPath = result.repo_path;
      const note = result.notes.length ? ` · ${result.notes.join("；")}` : "";
      const lithoNote = result.litho_ran && !result.human_docs_complete ? " · Litho 文档未完成" : "";
      setStatus(
        `初始化完成：索引 ${result.scan_files_written} 项，${TERMS.humanKnowledge} ${result.human_doc_count} 篇${lithoNote}${note}`,
        result.notes.length || lithoNote ? "idle" : "success",
        result.project_slug,
      );
      await refresh();
      await Promise.all([
        loadHumanDocs(result.project_slug),
        loadProjectOverview(result.project_slug),
        refreshKnowledgeRoot(result.project_slug),
      ]);
    } catch (e) {
      setStatus(String(e), "error");
    } finally {
      initBusy = false;
      initProgress = null;
      if (targetSlug) {
        setProjectTask(targetSlug, { repackBusy: false, lithoBusy: false, lithoProgress: "" });
      } else if (selectedProject) {
        setProjectTask(selectedProject, { repackBusy: false, lithoBusy: false, lithoProgress: "" });
      }
    }
  }

  async function triggerQuickRefresh() {
    if (!selectedRepoPath || !selectedProject) {
      setStatus("Select a project with a linked repository first.", "error");
      return;
    }
    if (quickRefreshBusy) return;
    const slug = selectedProject;
    quickRefreshBusy = true;
    setStatus("正在快速保鲜（扫描 + 索引 + Agent 知识资产）…", "progress", slug);
    try {
      const result = await runQuickRefresh(selectedRepoPath, slug);
      const note = result.notes.length ? ` · ${result.notes.join("；")}` : "";
      setStatus(
        `保鲜完成：新鲜度 ${result.freshness.overall_score}/100${note}`,
        result.freshness.overall_stale ? "idle" : "success",
        slug,
      );
      await loadProjectOverview(slug, { skipFreshness: true });
      if (selectedProject === slug && projectOverview) {
        projectOverview = mergeFreshnessIntoOverview(projectOverview, result.freshness);
      }
      freshnessLoading = false;
    } catch (e) {
      setStatus(String(e), "error");
    } finally {
      quickRefreshBusy = false;
    }
  }

  async function triggerAgentContextGeneration() {
    if (!selectedRepoPath || !selectedProject) {
      setStatus("Select a project with a linked repository first.", "error");
      return;
    }
    if (agentContextBusy) return;
    if (!llmStatus?.ready) {
        setStatus("请先在设置中配置 LLM。", "error");
      return;
    }
    const slug = selectedProject;
    agentContextBusy = true;
    setStatus("Generating Agent architecture context…", "progress", slug);
    try {
      await runAgentContextGeneration(selectedRepoPath, slug);
      setStatus("Agent context ready", "success");
      await Promise.all([loadProjectOverview(slug), loadHumanDocs(slug)]);
    } catch (e) {
      setStatus(String(e), "error");
    } finally {
      agentContextBusy = false;
    }
  }

  function openOverviewHumanDoc() {
    if (!selectedProject) return;
    const humanDir = projectOverview?.slug ?? selectedProject;
    void (async () => {
      const docs = await listHumanDocs(humanDir);
      const overview = docs.find((d) => d.relative_path.includes("1.概述"));
      if (overview) {
        activeTab = "knowledge";
        await openHumanDoc(overview);
      } else {
        setStatus(`尚未生成 1.概述.md，请先生成 ${TERMS.humanKnowledge}`, "error");
        activeTab = "knowledge";
      }
    })();
  }

  function openStructuredDocs() {
    if (!selectedProject) return;
    void (async () => {
      activeTab = "knowledge";
      docLoading = true;
      try {
        const docs = await listHumanDocs(selectedProject);
        humanDocs = docs;
        const agentMeta = docs.find(
          (d) => d.section === "agent" && d.relative_path === "agent/meta-inputs.md",
        );
        const structured = docs.filter((d) => d.section === "structured");
        const target = agentMeta ?? structured[0];
        if (target) {
          await openHumanDoc(target);
        } else {
          setStatus(
            "尚无结构化条目：在仓库根目录添加 mind-mesh-meta.json，然后生成 Agent 友好的知识资产",
            "error",
          );
        }
      } catch (e) {
        setStatus(String(e), "error");
      } finally {
        docLoading = false;
      }
    })();
  }

  async function openHumanDoc(doc: HumanDocEntry) {
    activeHumanPath = doc.path;
    docLoading = true;
    setStatus(`Opening ${doc.title}…`, "loading");
    try {
      activeDoc = await readDocument(doc.path);
      setStatus(`Viewing ${doc.title}`, "idle");
    } catch (e) {
      setStatus(String(e), "error");
    } finally {
      docLoading = false;
    }
  }

  async function openDocPath(path: string) {
    docLoading = true;
    try {
      activeDoc = await readDocument(path);
      activeHumanPath = path;
      deepWikiOpen = false;
    } catch (e) {
      setStatus(String(e), "error");
    } finally {
      docLoading = false;
    }
  }

  async function packAgentForSelected() {
    if (!selectedRepoPath || !selectedProject) return;
    setProjectTask(selectedProject, { repackBusy: true });
    setStatus("正在重建源码索引…", "progress", selectedProject);
    try {
      const pack = await packAgentAssets(selectedRepoPath, selectedProject);
      setStatus(
        `索引已更新：${pack.total_files} 个文件，约 ${pack.total_tokens} tokens`,
        "success",
      );
      if (selectedProject) await loadProjectOverview(selectedProject);
    } catch (e) {
      setStatus(String(e), "error");
    } finally {
      if (selectedProject) setProjectTask(selectedProject, { repackBusy: false });
    }
  }

  async function triggerHumanGeneration() {
    if (!selectedRepoPath || !selectedProject) {
      setStatus("Select a project with a linked repository path first.", "error");
      return;
    }
    if (!acpOk) {
      setStatus("ACP 代理未找到。请在设置中配置 ACP binary/command 并确保其在 PATH 上。", "error");
      return;
    }
    const slug = selectedProject;
    setProjectTask(slug, {
      lithoBusy: true,
      lithoProgress: `正在生成 ${TERMS.humanKnowledge}（Litho）…`,
    });
    try {
      await runLithoGeneration(selectedRepoPath, slug);
    } catch (e) {
      setStatus(String(e), "error");
      setProjectTask(slug, { lithoBusy: false, lithoProgress: "" });
    }
  }

  async function openKnowledgeSourceCitation(c: SourceCitation) {
    if (!selectedProject) return;

    try {
      const slice = await citationToSourceSlice(
        selectedProject,
        c,
        selectedRepoPath,
        readDocument,
      );
      setKnowledgeSource(selectedProject, slice);
    } catch (e) {
      setStatus(String(e), "error");
      setKnowledgeSource(selectedProject, null);
    }
  }

  function openDeepWiki(question?: string) {
    if (!selectedProject) {
      setStatus("Select a project before asking.", "error");
      return;
    }
    deepWikiInitialQuestion = question ?? null;
    deepWikiOpen = true;
  }

  function closeDeepWiki() {
    deepWikiOpen = false;
    deepWikiInitialQuestion = null;
  }

  async function runSearch() {
    const q = query.trim();
    if (!q) {
      hits = [];
      return;
    }
    docLoading = true;
    setStatus(`Searching for “${q}”…`, "loading");
    try {
      hits = await searchKnowledge(q, selectedProject ?? undefined);
      activeDoc = null;
      setStatus(`${hits.length} result(s)`, hits.length ? "success" : "idle");
    } catch (e) {
      setStatus(String(e), "error");
    } finally {
      docLoading = false;
    }
  }

  async function openHit(hit: SearchHit) {
    docLoading = true;
    try {
      activeDoc = await readDocument(hit.path);
      hits = [];
    } catch (e) {
      setStatus(String(e), "error");
    } finally {
      docLoading = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      activeTab = "knowledge";
      requestAnimationFrame(() => document.getElementById("search-input")?.focus());
    }
    if (e.key === "Enter" && document.activeElement?.id === "search-input") {
      runSearch();
    }
  }

  onMount(() => {
    refresh();
    window.addEventListener("keydown", onKeydown);

    let unlistenProgress: (() => void) | undefined;
    let unlistenDone: (() => void) | undefined;
    let unlistenInitProgress: (() => void) | undefined;

    void (async () => {
      unlistenInitProgress = await listen<{ project_slug: string; stage: string; message: string }>(
        "project-init-progress",
        (ev) => {
          const { project_slug, message } = ev.payload;
          initProgress = message;
          if (ev.payload.stage === "human_docs") {
            setProjectTask(project_slug, { lithoBusy: true, lithoProgress: message });
          } else if (ev.payload.stage === "scan") {
            setProjectTask(project_slug, { repackBusy: true });
          } else if (ev.payload.stage === "done") {
            setProjectTask(project_slug, { lithoBusy: false, lithoProgress: "", repackBusy: false });
          }
        },
      );
      unlistenProgress = await listen<{ project_slug: string; stage: string; message: string }>(
        "litho-progress",
        (ev) => {
          const { project_slug, stage, message } = ev.payload;
          const label = `[${stage}] ${message}`;
          setProjectTask(project_slug, { lithoProgress: label });
        },
      );
      unlistenDone = await listen<{
        project_slug: string;
        result: { human_doc_count: number; human_docs_complete: boolean };
      }>(
        "litho-done",
        async (ev) => {
          const { project_slug, result } = ev.payload;
          setProjectTask(project_slug, { lithoBusy: false, lithoProgress: "" });
          const count = result.human_doc_count;
          const complete = result.human_docs_complete;
          const msg = !complete
            ? count === 0
              ? `Litho 已完成，但未写入 ${TERMS.humanKnowledge}（${project_slug}）`
              : `${TERMS.humanKnowledge} 未完成（${project_slug}，${count} 篇）`
            : `${TERMS.humanKnowledge} 已就绪（${project_slug}，${count} 篇）`;
          if (selectedProject === project_slug) {
            setStatus(msg, complete ? "success" : count === 0 ? "error" : "idle");
            await loadHumanDocs(project_slug);
            await loadProjectOverview(project_slug);
          }
        },
      );
    })();

    return () => {
      window.removeEventListener("keydown", onKeydown);
      unlistenProgress?.();
      unlistenDone?.();
      unlistenInitProgress?.();
    };
  });
</script>

<div class="flex h-screen flex-col">
  <header class="relative z-50 flex shrink-0 items-center gap-3 overflow-x-auto border-b border-white/10 bg-[#14171c] px-4 py-2.5">
    <div class="flex min-w-0 shrink items-center gap-3">
      <div class="shrink-0">
        <h1 class="text-base font-semibold tracking-tight">MindMesh</h1>
      </div>

      <ProjectSelector
        {projects}
        selectedSlug={selectedProject}
        open={projectPickerOpen}
        addBusy={initBusy}
        ontoggle={() => (projectPickerOpen = !projectPickerOpen)}
        onselect={selectProject}
        onadd={addProject}
        onremove={removeProjectFromList}
        onopenFolder={openProjectFolder}
      />

      <MainNavTabs
        active={activeTab}
        disabled={!selectedProject && projects.length === 0}
        onchange={(tab) => {
          activeTab = tab;
          if (tab === "knowledge" && !selectedProject && projects.length > 0) {
            void selectProject(projects[0]);
          }
        }}
      />
    </div>

    <div class="ml-auto flex shrink-0 items-center gap-2">
      {#if showStatusBar}
        <StatusBanner message={statusMessage} kind={statusKind} detail={statusDetail} />
      {/if}
      <button
        type="button"
        class="shrink-0 rounded-lg border border-white/10 px-2.5 py-1.5 text-xs text-white/70 hover:bg-white/5"
        title="术语说明"
        aria-label="术语说明"
        onclick={() => (helpOpen = true)}
      >
        ？
      </button>
      <button
        type="button"
        class="shrink-0 rounded-lg border border-white/10 px-2.5 py-1.5 text-xs text-white/70 hover:bg-white/5"
        title="设置"
        aria-label="Settings"
        onclick={() => (settingsOpen = true)}
      >
        ⚙ 设置
      </button>
    </div>
  </header>

  {#if showTaskProgressBar && selectedProject}
    <TaskProgressBar
      projectSlug={selectedProject}
      stage={initBusy ? "初始化" : lithoProgressParts.stage}
      message={initBusy && initProgress ? initProgress : lithoProgressParts.message}
    />
  {/if}

  <div class="flex min-h-0 flex-1 flex-col" class:hidden={activeTab !== "overview"}>
    <ProjectOverviewPanel
      overview={projectOverview}
      loading={overviewLoading}
      {acpOk}
      llmReady={llmStatus?.ready ?? false}
      {agentContextBusy}
      lithoBusy={lithoBusy}
      repackBusy={repackBusy}
      {initBusy}
      {initProgress}
      {staleProjects}
      onOpenKnowledge={() => (activeTab = "knowledge")}
      onOpenSdd={() => (activeTab = "sdd")}
      onOpenEnv={() => (activeTab = "env")}
      onOpenAsk={() => openDeepWiki()}
      onGenerateHuman={triggerHumanGeneration}
      onGenerateAgentContext={triggerAgentContextGeneration}
      onRepack={packAgentForSelected}
      onInitializeProject={triggerProjectInitialization}
      onOpenPath={openFolderPath}
      onOpenArchitectureDoc={projectOverview?.agent_context.ready ? openArchitectureDoc : undefined}
      onOpenHumanOverview={projectOverview?.litho.has_human_docs ? openOverviewHumanDoc : undefined}
      onOpenStructured={openStructuredDocs}
      quickRefreshBusy={quickRefreshBusy}
      freshnessLoading={freshnessLoading}
      onQuickRefresh={triggerQuickRefresh}
    />
  </div>

  <div class="flex min-h-0 flex-1 flex-col" class:hidden={activeTab !== "sdd"}>
    <SddWorkflowPanel
      projectSlug={selectedProject}
      repoPath={selectedRepoPath}
      {acpOk}
      llmReady={llmStatus?.ready ?? false}
      onStatus={(message, kind) => setStatus(message, kind)}
    />
  </div>

  {#if activeTab === "env"}
    <div class="flex min-h-0 flex-1 flex-col">
      <EnvIntegratePanel
        repoPath={selectedRepoPath}
        onStatus={(message, kind) => setStatus(message, kind)}
        onIntegrated={() => {
          if (selectedProject) void loadProjectOverview(selectedProject);
        }}
      />
    </div>
  {/if}

  <div class="flex min-h-0 flex-1 flex-col" class:hidden={activeTab !== "knowledge"}>
      <div class="flex shrink-0 items-center gap-2 border-b border-white/10 bg-[#14171c]/80 px-4 py-2">
        <input
          id="search-input"
          class="min-w-0 flex-1 rounded-lg border border-white/10 bg-white/5 px-3 py-1.5 text-sm outline-none focus:border-indigo-500"
          placeholder={`搜索${TERMS.knowledgeTab}… (⌘K)`}
          bind:value={query}
        />
        <button
          type="button"
          class="shrink-0 rounded-lg bg-white/10 px-3 py-1.5 text-sm hover:bg-white/15 disabled:opacity-50"
          disabled={docLoading}
          onclick={runSearch}
        >
          搜索
        </button>
        {#if selectedProject}
          <div class="flex shrink-0 items-center gap-2 border-l border-white/10 pl-3">
            <button
              type="button"
              class="rounded-lg border border-white/10 px-2.5 py-1.5 text-xs hover:bg-white/5 disabled:opacity-50"
              disabled={repackBusy || lithoBusy || !selectedRepoPath}
              onclick={packAgentForSelected}
            >
              {repackBusy ? "重建中…" : "重建源码索引"}
            </button>
            <button
              type="button"
              class="rounded-lg border border-white/10 px-2.5 py-1.5 text-xs hover:bg-white/5 disabled:opacity-50"
              disabled={repackBusy || lithoBusy || !selectedRepoPath || !acpOk}
              onclick={triggerHumanGeneration}
              title={!acpOk ? "请先在设置中配置 ACP 代理" : undefined}
            >
              {generateLabel(TERMS.humanKnowledge, lithoBusy)}
            </button>
          </div>
        {/if}
      </div>

      <div class="flex min-h-0 flex-1">
        <aside class="flex w-60 shrink-0 flex-col border-r border-white/10 bg-[#14171c]">
          <HumanDocTree
            docs={humanDocs}
            activePath={activeHumanPath}
            loading={humanDocsLoading}
            onselect={openHumanDoc}
          />
          <div class="mt-auto border-t border-white/10 px-3 py-2 text-[10px] text-white/35">
            <div class="truncate" title={knowledgeRoot || selectedRepoPath || "—"}>
              📁 {knowledgeRoot || (selectedRepoPath ? `${selectedRepoPath}/.mind-mesh` : "—")}
            </div>
            <div class="mt-1">ACP {acpOk ? "✓" : "✗"} · LLM {llmStatus?.ready ? "✓" : "✗"}</div>
          </div>
        </aside>

        <main class="flex min-w-0 flex-1 flex-col">
          {#if docLoading}
            <div class="flex flex-1 flex-col items-center justify-center gap-3 text-sm text-white/40">
              <span class="inline-block h-8 w-8 animate-spin rounded-full border-2 border-indigo-400 border-t-transparent"></span>
              <span>Loading document…</span>
            </div>
          {:else if activeDoc}
            <KnowledgeArticle
              body={activeDoc.body}
              path={activeDoc.path}
              repoPath={selectedRepoPath}
              onSourceClick={openKnowledgeSourceCitation}
            />
    {:else}
            <div class="flex-1 overflow-y-auto">
              {#if hits.length > 0}
              <ul class="p-4">
                {#each hits as hit}
                  <li>
                    <button
                      type="button"
                      class="mb-2 w-full rounded-lg border border-white/10 bg-white/[0.03] px-4 py-3 text-left hover:bg-white/[0.06]"
                      onclick={() => openHit(hit)}
                    >
                      <div class="flex items-center gap-2">
                        <span class="rounded bg-white/10 px-1.5 py-0.5 text-[10px] uppercase">{hit.doc_type}</span>
                        <span class="font-medium">{hit.title ?? hit.path}</span>
                      </div>
                      <p class="mt-1 text-sm text-white/50">{hit.snippet}</p>
                    </button>
                  </li>
                {/each}
              </ul>
              {:else}
                <div class="flex h-full flex-col items-center justify-center gap-2 px-6 text-center text-white/40">
                  <p class="text-lg text-white/60">
                    {selectedProject ? "从左侧目录选择文档" : "添加或选择项目以浏览知识资产"}
                  </p>
                  <p class="text-sm">阅读文档后，可在底部问答栏就当前项目提问。</p>
                </div>
              {/if}
            </div>
          {/if}

          <AskBar
            disabled={!selectedProject}
            disabledReason={!selectedProject ? "请先选择项目" : null}
            placeholder={activeDoc
              ? `就「${activeHumanPath?.split("/").pop() ?? "当前文档"}」提问…`
              : "就当前项目提问…"}
            onask={(q) => openDeepWiki(q)}
          />
        </main>
      </div>
  </div>
</div>

<DeepWikiPanel
  open={deepWikiOpen}
  projectSlug={selectedProject}
  projectName={selectedProjectMeta?.name ?? null}
  repoPath={selectedRepoPath}
  messages={currentMessages}
  initialQuestion={deepWikiInitialQuestion}
  sourceSlice={currentDeepWikiSource}
  onclose={closeDeepWiki}
  onmessageschange={(update) => {
    if (!selectedProject) return;
    updateChat(selectedProject, update);
  }}
  onsourcechange={(slice) => {
    if (!selectedProject) return;
    setDeepWikiSource(selectedProject, slice);
  }}
  onopenDoc={openDocPath}
/>

<SourceDrawer
  open={Boolean(knowledgeSourceSlice)}
  slice={knowledgeSourceSlice}
  repoPath={selectedRepoPath}
  onclose={() => selectedProject && setKnowledgeSource(selectedProject, null)}
  onSourceClick={openKnowledgeSourceCitation}
/>

<HelpPanel open={helpOpen} onclose={() => (helpOpen = false)} />

<SettingsPanel
  open={settingsOpen}
  onclose={() => (settingsOpen = false)}
  onsaved={(status) => {
    llmStatus = status;
    setStatus(status.ready ? "LLM settings saved" : status.message, status.ready ? "success" : "error");
  }}
/>
