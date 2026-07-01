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
    getModelSettings,
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
    saveProjectRemark,
    searchKnowledge,
  } from "./lib/api";
  import { mergeFreshnessIntoOverview } from "./lib/mergeFreshness";
  import { usesNativeLlm, normalizeAgentExecution } from "./lib/agentExecution";
  import { parseAskSlashCommand } from "./lib/askSlashCommands";
  import { generateLabel, TERMS, UI_MESSAGES } from "./lib/terminology";
  import { citationToSourceSlice, createPendingSourceSlice } from "./lib/resolveSource";
  import { setStatus, status } from "./lib/stores/status.svelte";
  import {
    chatSessions,
    deepWikiSources,
    knowledgeSources,
    setDeepWikiSource,
    setKnowledgeSource,
    updateChat,
  } from "./lib/stores/chat.svelte";
  import {
    currentTask,
    project,
    selectedProjectMeta,
    setProjectTask,
  } from "./lib/stores/project.svelte";
  import type {
    ChatMessage,
    HumanDocEntry,
    KnowledgeDoc,
    ProjectSummary,
    SearchHit,
    SourceCitation,
    SourceSlice,
  } from "./lib/types";

  const hybridNativeLlm = $derived(usesNativeLlm(project.agentExecution));

  const selectedProjectMetaDerived = $derived(selectedProjectMeta());
  const currentTaskDerived = $derived(currentTask());
  const currentMessages = $derived(
    project.selectedSlug ? (chatSessions[project.selectedSlug] ?? []) : [],
  );
  const currentDeepWikiSource = $derived(
    project.selectedSlug ? (deepWikiSources[project.selectedSlug] ?? null) : null,
  );
  const knowledgeSourceSlice = $derived(
    project.selectedSlug ? (knowledgeSources[project.selectedSlug] ?? null) : null,
  );
  let knowledgeSourceLoadId = 0;
  const repackBusy = $derived(currentTaskDerived.repackBusy);
  const lithoBusy = $derived(currentTaskDerived.lithoBusy);
  const lithoProgress = $derived(currentTaskDerived.lithoProgress);

  async function refreshKnowledgeRoot(slug?: string | null) {
    const target = slug ?? project.selectedSlug;
    if (!target) {
      project.knowledgeRoot = "";
      return;
    }
    try {
      project.knowledgeRoot = await getKnowledgeRoot(target);
    } catch {
      project.knowledgeRoot = "";
    }
  }

  async function refresh() {
    setStatus("正在刷新项目列表…", "loading");
    try {
      const settings = await getModelSettings();
      project.agentExecution = normalizeAgentExecution(settings.acp?.agent_execution);
      [project.projects, project.staleProjects, project.llmStatus] = await Promise.all([
        listProjects(),
        listStaleProjects(),
        checkLlm(),
      ]);
      void checkAcp().then((ok) => {
        project.acpOk = ok;
      });
      if (!project.selectedSlug && project.projects.length > 0) {
        await selectProject(project.projects[0]);
      } else {
        await refreshKnowledgeRoot();
      }
      setStatus(`已索引 ${project.projects.length} 个项目`, "success");
    } catch (e) {
      setStatus(String(e), "error");
    }
  }

  async function loadProjectOverviewFreshness(slug: string, repoPath: string | null | undefined) {
    if (!repoPath) {
      project.freshnessLoading = false;
      return;
    }
    const requestSlug = slug;
    project.freshnessLoading = true;
    try {
      const cached = await readProjectFreshnessCached(slug);
      if (cached && project.selectedSlug === requestSlug && project.projectOverview) {
        project.projectOverview = mergeFreshnessIntoOverview(project.projectOverview, cached);
        project.freshnessLoading = false;
      }

      window.setTimeout(() => {
        void (async () => {
          try {
            const freshness = await computeFreshness(slug, repoPath);
            if (project.selectedSlug === requestSlug && project.projectOverview) {
              project.projectOverview = mergeFreshnessIntoOverview(
                project.projectOverview,
                freshness,
              );
            }
          } catch {
            /* keep cached freshness */
          } finally {
            if (project.selectedSlug === requestSlug) {
              project.freshnessLoading = false;
            }
          }
        })();
      }, 2000);
    } catch {
      project.freshnessLoading = false;
    }
  }

  async function loadProjectOverview(slug: string, opts?: { skipFreshness?: boolean }) {
    project.overviewLoading = true;
    try {
      project.projectOverview = await getProjectOverview(slug);
      if (project.projectOverview?.repo_path && !opts?.skipFreshness) {
        void loadProjectOverviewFreshness(slug, project.projectOverview.repo_path);
      } else {
        project.freshnessLoading = false;
      }
    } catch {
      project.projectOverview = null;
      project.freshnessLoading = false;
    } finally {
      project.overviewLoading = false;
    }
  }

  async function loadHumanDocs(slug: string) {
    project.humanDocsLoading = true;
    try {
      project.humanDocs = await listHumanDocs(slug);
    } catch {
      project.humanDocs = [];
    } finally {
      project.humanDocsLoading = false;
    }
  }

  async function addProject() {
    project.pickerOpen = false;
    if (project.initBusy) return;
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

  async function removeProjectFromList(item: ProjectSummary) {
    project.pickerOpen = false;
    try {
      await removeProject(item.slug);
      if (project.selectedSlug === item.slug) {
        project.selectedSlug = null;
        project.selectedRepoPath = null;
        project.projectOverview = null;
        project.activeDoc = null;
        project.activeHumanPath = null;
        project.humanDocs = [];
        project.hits = [];
      }
      await refresh();
      setStatus(`已从列表移除：${item.name}`, "success");
    } catch (e) {
      setStatus(String(e), "error");
    }
  }

  async function selectProject(item: ProjectSummary) {
    project.pickerOpen = false;
    if (project.selectedSlug !== item.slug) {
      project.activeDoc = null;
      project.activeHumanPath = null;
      project.hits = [];
      project.deepWikiInitialQuestion = null;
    }
    project.selectedSlug = item.slug;
    project.selectedRepoPath = item.repo_path ?? null;
    if (!project.selectedRepoPath) {
      try {
        const doc = await readDocument(item.path);
        const source = doc.frontmatter.source;
        project.selectedRepoPath = typeof source === "string" ? source : null;
      } catch {
        project.selectedRepoPath = null;
      }
    }
    setStatus(`项目：${item.name}`, "idle", item.slug);
    void loadHumanDocs(item.slug);
    void loadProjectOverview(item.slug);
    void refreshKnowledgeRoot(item.slug);
  }

  async function openFolderPath(path: string) {
    try {
      await openRepoFolder(path);
    } catch (e) {
      setStatus(UI_MESSAGES.openFolderFailed(e), "error");
    }
  }

  async function openProjectFolder(item: ProjectSummary) {
    const repo = item.repo_path ?? (item.slug === project.selectedSlug ? project.selectedRepoPath : null);
    if (!repo) {
      setStatus(`项目 ${item.name} 未关联仓库路径`, "error");
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
    Boolean(project.selectedSlug && (lithoProgress || (project.initBusy && project.initProgress))),
  );

  const showStatusBar = $derived(
    !showTaskProgressBar && (status.kind !== "idle" || status.message !== "就绪"),
  );

  function openArchitectureDoc() {
    const slug = project.selectedSlug;
    const docPath = project.projectOverview?.agent_context.path;
    if (!docPath || !slug) return;
    void (async () => {
      project.activeTab = "knowledge";
      project.docLoading = true;
      try {
        await loadHumanDocs(slug);
        project.activeDoc = await readDocument(docPath);
        project.activeHumanPath = docPath;
      } catch (e) {
        setStatus(String(e), "error");
      } finally {
        project.docLoading = false;
      }
    })();
  }

  async function triggerProjectInitialization(repoPath: string, slug?: string) {
    if (project.initBusy) return;
    project.initBusy = true;
    project.initProgress = "正在扫描仓库…";
    const targetSlug = slug ?? null;
    if (targetSlug) {
      project.selectedSlug = targetSlug;
      project.selectedRepoPath = repoPath;
      setProjectTask(targetSlug, { repackBusy: true, lithoBusy: false, lithoProgress: "" });
    }
    try {
      const result = await initializeProject(repoPath, slug);
      project.selectedSlug = result.project_slug;
      project.selectedRepoPath = result.repo_path;
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
      project.initBusy = false;
      project.initProgress = null;
      if (targetSlug) {
        setProjectTask(targetSlug, { repackBusy: false, lithoBusy: false, lithoProgress: "" });
      } else if (project.selectedSlug) {
        setProjectTask(project.selectedSlug, { repackBusy: false, lithoBusy: false, lithoProgress: "" });
      }
    }
  }

  async function triggerQuickRefresh() {
    if (!project.selectedRepoPath || !project.selectedSlug) {
      setStatus(UI_MESSAGES.selectProjectWithRepo, "error");
      return;
    }
    if (project.quickRefreshBusy) return;
    const slug = project.selectedSlug;
    project.quickRefreshBusy = true;
    setStatus("正在快速保鲜（扫描 + 索引 + Agent 知识资产）…", "progress", slug);
    try {
      const result = await runQuickRefresh(project.selectedRepoPath, slug);
      const note = result.notes.length ? ` · ${result.notes.join("；")}` : "";
      setStatus(
        `保鲜完成：新鲜度 ${result.freshness.overall_score}/100${note}`,
        result.freshness.overall_stale ? "idle" : "success",
        slug,
      );
      await loadProjectOverview(slug, { skipFreshness: true });
      if (project.selectedSlug === slug && project.projectOverview) {
        project.projectOverview = mergeFreshnessIntoOverview(project.projectOverview, result.freshness);
      }
      project.freshnessLoading = false;
    } catch (e) {
      setStatus(String(e), "error");
    } finally {
      project.quickRefreshBusy = false;
    }
  }

  async function triggerAgentContextGeneration() {
    if (!project.selectedRepoPath || !project.selectedSlug) {
      setStatus(UI_MESSAGES.selectProjectWithRepo, "error");
      return;
    }
    if (project.agentContextBusy) return;
    if (!project.acpOk) {
      setStatus("请先在设置中配置 ACP 代理。", "error");
      return;
    }
    if (hybridNativeLlm && !project.llmStatus?.ready) {
      setStatus("请先在设置中配置 LLM。", "error");
      return;
    }
    const slug = project.selectedSlug;
    project.agentContextBusy = true;
    setStatus(UI_MESSAGES.agentContextGenerating, "progress", slug);
    try {
      await runAgentContextGeneration(project.selectedRepoPath, slug);
      setStatus(UI_MESSAGES.agentContextReady, "success");
      await Promise.all([loadProjectOverview(slug), loadHumanDocs(slug)]);
    } catch (e) {
      setStatus(String(e), "error");
    } finally {
      project.agentContextBusy = false;
    }
  }

  function openOverviewHumanDoc() {
    if (!project.selectedSlug) return;
    const humanDir = project.projectOverview?.slug ?? project.selectedSlug;
    void (async () => {
      const docs = await listHumanDocs(humanDir);
      const overview = docs.find((d) => d.relative_path.includes("1.概述"));
      if (overview) {
        project.activeTab = "knowledge";
        await openHumanDoc(overview);
      } else {
        setStatus(`尚未生成 1.概述.md，请先生成 ${TERMS.humanKnowledge}`, "error");
        project.activeTab = "knowledge";
      }
    })();
  }

  function openStructuredDocs() {
    const slug = project.selectedSlug;
    if (!slug) return;
    void (async () => {
      project.activeTab = "knowledge";
      project.docLoading = true;
      try {
        const docs = await listHumanDocs(slug);
        project.humanDocs = docs;
        const agentMeta = docs.find(
          (d) => d.section === "agent" && d.relative_path === "agent/meta-inputs.md",
        );
        const structured = docs.filter((d) => d.section === "structured");
        const target = agentMeta ?? structured[0];
        if (target) {
          await openHumanDoc(target);
        } else {
          setStatus(
            "尚无结构化条目：在仓库根目录添加 terrain-meta.json，然后生成 Agent 友好的知识资产",
            "error",
          );
        }
      } catch (e) {
        setStatus(String(e), "error");
      } finally {
        project.docLoading = false;
      }
    })();
  }

  async function openHumanDoc(doc: HumanDocEntry) {
    project.activeHumanPath = doc.path;
    project.docLoading = true;
    setStatus(`Opening ${doc.title}…`, "loading");
    try {
      project.activeDoc = await readDocument(doc.path);
      setStatus(`Viewing ${doc.title}`, "idle");
    } catch (e) {
      setStatus(String(e), "error");
    } finally {
      project.docLoading = false;
    }
  }

  async function openDocPath(path: string) {
    project.docLoading = true;
    try {
      project.activeDoc = await readDocument(path);
      project.activeHumanPath = path;
      project.deepWikiOpen = false;
    } catch (e) {
      setStatus(String(e), "error");
    } finally {
      project.docLoading = false;
    }
  }

  async function packAgentForSelected() {
    if (!project.selectedRepoPath || !project.selectedSlug) return;
    setProjectTask(project.selectedSlug, { repackBusy: true });
    setStatus("正在重建源码索引…", "progress", project.selectedSlug);
    try {
      const pack = await packAgentAssets(project.selectedRepoPath, project.selectedSlug);
      setStatus(
        `索引已更新：${pack.total_files} 个文件，约 ${pack.total_tokens} tokens`,
        "success",
      );
      if (project.selectedSlug) await loadProjectOverview(project.selectedSlug);
    } catch (e) {
      setStatus(String(e), "error");
    } finally {
      if (project.selectedSlug) setProjectTask(project.selectedSlug, { repackBusy: false });
    }
  }

  async function triggerHumanGeneration() {
    if (!project.selectedRepoPath || !project.selectedSlug) {
      setStatus(UI_MESSAGES.selectProjectWithRepoPath, "error");
      return;
    }
    if (!project.acpOk) {
      setStatus("ACP 代理未找到。请在设置中配置 ACP binary/command 并确保其在 PATH 上。", "error");
      return;
    }
    const slug = project.selectedSlug;
    setProjectTask(slug, {
      lithoBusy: true,
      lithoProgress: `正在生成 ${TERMS.humanKnowledge}（Litho）…`,
    });
    try {
      await runLithoGeneration(project.selectedRepoPath, slug);
    } catch (e) {
      setStatus(String(e), "error");
      setProjectTask(slug, { lithoBusy: false, lithoProgress: "" });
    }
  }

  async function openKnowledgeSourceCitation(c: SourceCitation) {
    if (!project.selectedSlug) return;

    const slug = project.selectedSlug;
    const loadId = ++knowledgeSourceLoadId;
    setKnowledgeSource(slug, createPendingSourceSlice(c, project.selectedRepoPath));

    try {
      const slice = await citationToSourceSlice(
        slug,
        c,
        project.selectedRepoPath,
        readDocument,
      );
      if (loadId !== knowledgeSourceLoadId) return;
      setKnowledgeSource(slug, slice);
    } catch (e) {
      if (loadId !== knowledgeSourceLoadId) return;
      setStatus(String(e), "error");
      setKnowledgeSource(slug, {
        ...createPendingSourceSlice(c, project.selectedRepoPath),
        status: "error",
        content: String(e),
      });
    }
  }

  function clearChatHistory() {
    if (!project.selectedSlug) return;
    updateChat(project.selectedSlug, () => []);
    setStatus("对话历史已清空", "success");
  }

  function handleAskInput(q: string) {
    if (!project.selectedSlug) {
      setStatus("请先选择项目", "error");
      return;
    }
    if (parseAskSlashCommand(q)?.type === "clear") {
      clearChatHistory();
      return;
    }
    openDeepWiki(q);
  }

  function openDeepWiki(question?: string) {
    if (!project.selectedSlug) {
      setStatus("请先选择项目", "error");
      return;
    }
    if (question && parseAskSlashCommand(question)?.type === "clear") {
      clearChatHistory();
      project.deepWikiOpen = true;
      project.deepWikiInitialQuestion = null;
      return;
    }
    if (!project.acpOk) {
      setStatus("请先在设置中配置 ACP 代理。", "error");
      return;
    }
    project.deepWikiInitialQuestion = question ?? null;
    project.deepWikiOpen = true;
  }

  function closeDeepWiki() {
    project.deepWikiOpen = false;
    project.deepWikiInitialQuestion = null;
  }

  async function runSearch() {
    const q = project.query.trim();
    if (!q) {
      project.hits = [];
      return;
    }
    project.docLoading = true;
    setStatus(`Searching for “${q}”…`, "loading");
    try {
      project.hits = await searchKnowledge(q, project.selectedSlug ?? undefined);
      project.activeDoc = null;
      setStatus(`${project.hits.length} result(s)`, project.hits.length ? "success" : "idle");
    } catch (e) {
      setStatus(String(e), "error");
    } finally {
      project.docLoading = false;
    }
  }

  async function openHit(hit: SearchHit) {
    project.docLoading = true;
    try {
      project.activeDoc = await readDocument(hit.path);
      project.hits = [];
    } catch (e) {
      setStatus(String(e), "error");
    } finally {
      project.docLoading = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      project.activeTab = "knowledge";
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
          project.initProgress = message;
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
          if (project.selectedSlug === project_slug) {
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
        <h1 class="text-base font-semibold tracking-tight">Terrain</h1>
      </div>

      <ProjectSelector
        projects={project.projects}
        selectedSlug={project.selectedSlug}
        open={project.pickerOpen}
        addBusy={project.initBusy}
        ontoggle={() => (project.pickerOpen = !project.pickerOpen)}
        onselect={selectProject}
        onadd={addProject}
        onremove={removeProjectFromList}
        onopenFolder={openProjectFolder}
      />

      <MainNavTabs
        active={project.activeTab}
        disabled={!project.selectedSlug && project.projects.length === 0}
        onchange={(tab) => {
          project.activeTab = tab;
          if (tab === "knowledge" && !project.selectedSlug && project.projects.length > 0) {
            void selectProject(project.projects[0]);
          }
        }}
      />
    </div>

    <div class="ml-auto flex shrink-0 items-center gap-2">
      {#if showStatusBar}
        <StatusBanner message={status.message} kind={status.kind} detail={status.detail} />
      {/if}
      <button
        type="button"
        class="shrink-0 rounded-lg border border-white/10 px-2.5 py-1.5 text-xs text-white/70 hover:bg-white/5"
        title="术语说明"
        aria-label="术语说明"
        onclick={() => (project.helpOpen = true)}
      >
        ？
      </button>
      <button
        type="button"
        class="shrink-0 rounded-lg border border-white/10 px-2.5 py-1.5 text-xs text-white/70 hover:bg-white/5"
        title="设置"
        aria-label="Settings"
        onclick={() => (project.settingsOpen = true)}
      >
        ⚙ 设置
      </button>
    </div>
  </header>

  {#if showTaskProgressBar && project.selectedSlug}
    <TaskProgressBar
      projectSlug={project.selectedSlug}
      stage={project.initBusy ? "初始化" : lithoProgressParts.stage}
      message={project.initBusy && project.initProgress ? project.initProgress : lithoProgressParts.message}
    />
  {/if}

  <div class="flex min-h-0 flex-1 flex-col" class:hidden={project.activeTab !== "overview"}>
    <ProjectOverviewPanel
      overview={project.projectOverview}
      loading={project.overviewLoading}
      acpOk={project.acpOk}
      llmReady={project.llmStatus?.ready ?? false}
      {hybridNativeLlm}
      agentContextBusy={project.agentContextBusy}
      lithoBusy={lithoBusy}
      repackBusy={repackBusy}
      initBusy={project.initBusy}
      initProgress={project.initProgress}
      staleProjects={project.staleProjects}
      onOpenKnowledge={() => (project.activeTab = "knowledge")}
      onOpenEnv={() => (project.activeTab = "env")}
      onOpenSettings={() => (project.settingsOpen = true)}
      onOpenAsk={() => openDeepWiki()}
      onGenerateHuman={triggerHumanGeneration}
      onGenerateAgentContext={triggerAgentContextGeneration}
      onRepack={packAgentForSelected}
      onInitializeProject={triggerProjectInitialization}
      onOpenPath={openFolderPath}
      onOpenArchitectureDoc={project.projectOverview?.agent_context.ready ? openArchitectureDoc : undefined}
      onOpenHumanOverview={project.projectOverview?.litho.has_human_docs ? openOverviewHumanDoc : undefined}
      onOpenStructured={openStructuredDocs}
      quickRefreshBusy={project.quickRefreshBusy}
      freshnessLoading={project.freshnessLoading}
      onQuickRefresh={triggerQuickRefresh}
      onSaveProjectRemark={async (remark) => {
        if (!project.selectedSlug) return;
        const prevFreshness = project.projectOverview?.freshness;
        const updated = await saveProjectRemark(project.selectedSlug, remark);
        project.projectOverview =
          prevFreshness && !updated.freshness
            ? mergeFreshnessIntoOverview(updated, prevFreshness)
            : updated;
      }}
    />
  </div>

  <div class="flex min-h-0 flex-1 flex-col" class:hidden={project.activeTab !== "sdd"}>
    {#if project.activeTab === "sdd"}
    <SddWorkflowPanel
      projectSlug={project.selectedSlug}
      repoPath={project.selectedRepoPath}
      acpOk={project.acpOk}
      llmReady={project.llmStatus?.ready ?? false}
      {hybridNativeLlm}
      onStatus={(message, kind) => setStatus(message, kind)}
    />
    {/if}
  </div>

  {#if project.activeTab === "env"}
    <div class="flex min-h-0 flex-1 flex-col">
      <EnvIntegratePanel
        repoPath={project.selectedRepoPath}
        onStatus={(message, kind) => setStatus(message, kind)}
        onIntegrated={() => {
          if (project.selectedSlug) void loadProjectOverview(project.selectedSlug);
        }}
      />
    </div>
  {/if}

  <div class="flex min-h-0 flex-1 flex-col" class:hidden={project.activeTab !== "knowledge"}>
    {#if project.activeTab === "knowledge"}
      <div class="flex shrink-0 items-center gap-2 border-b border-white/10 bg-[#14171c]/80 px-4 py-2">
        <input
          id="search-input"
          class="min-w-0 flex-1 rounded-lg border border-white/10 bg-white/5 px-3 py-1.5 text-sm outline-none focus:border-indigo-500"
          placeholder={`搜索${TERMS.knowledgeTab}… (⌘K)`}
          bind:value={project.query}
        />
        <button
          type="button"
          class="shrink-0 rounded-lg bg-white/10 px-3 py-1.5 text-sm hover:bg-white/15 disabled:opacity-50"
          disabled={project.docLoading}
          onclick={runSearch}
        >
          搜索
        </button>
        {#if project.selectedSlug}
          <div class="flex shrink-0 items-center gap-2 border-l border-white/10 pl-3">
            <button
              type="button"
              class="rounded-lg border border-white/10 px-2.5 py-1.5 text-xs hover:bg-white/5 disabled:opacity-50"
              disabled={repackBusy || lithoBusy || !project.selectedRepoPath}
              onclick={packAgentForSelected}
            >
              {repackBusy ? "重建中…" : "重建源码索引"}
            </button>
            <button
              type="button"
              class="rounded-lg border border-white/10 px-2.5 py-1.5 text-xs hover:bg-white/5 disabled:opacity-50"
              disabled={repackBusy || lithoBusy || !project.selectedRepoPath || !project.acpOk}
              onclick={triggerHumanGeneration}
              title={!project.acpOk ? "请先在设置中配置 ACP 代理" : undefined}
            >
              {generateLabel(TERMS.humanKnowledge, lithoBusy)}
            </button>
          </div>
        {/if}
      </div>

      <div class="flex min-h-0 flex-1">
        <aside class="flex w-60 shrink-0 flex-col border-r border-white/10 bg-[#14171c]">
          <HumanDocTree
            docs={project.humanDocs}
            activePath={project.activeHumanPath}
            loading={project.humanDocsLoading}
            onselect={openHumanDoc}
          />
          <div class="mt-auto border-t border-white/10 px-3 py-2 text-[10px] text-white/35">
            <div class="truncate" title={project.knowledgeRoot || project.selectedRepoPath || "—"}>
              📁 {project.knowledgeRoot || (project.selectedRepoPath ? `${project.selectedRepoPath}/.terrain` : "—")}
            </div>
            <div class="mt-1">
              ACP {project.acpOk ? "✓" : "✗"}{#if hybridNativeLlm} · LLM {project.llmStatus?.ready ? "✓" : "✗"}{/if}
            </div>
          </div>
        </aside>

        <main class="flex min-w-0 flex-1 flex-col">
          {#if project.docLoading}
            <div class="flex flex-1 flex-col items-center justify-center gap-3 text-sm text-white/40">
              <span class="inline-block h-8 w-8 animate-spin rounded-full border-2 border-indigo-400 border-t-transparent"></span>
              <span>{UI_MESSAGES.loadingDocument}</span>
            </div>
          {:else if project.activeDoc}
            <KnowledgeArticle
              body={project.activeDoc.body}
              path={project.activeDoc.path}
              repoPath={project.selectedRepoPath}
              onSourceClick={openKnowledgeSourceCitation}
            />
    {:else}
            <div class="flex-1 overflow-y-auto">
              {#if project.hits.length > 0}
              <ul class="p-4">
                {#each project.hits as hit}
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
                    {project.selectedSlug ? "从左侧目录选择文档" : "添加或选择项目以浏览知识资产"}
                  </p>
                  <p class="text-sm">阅读文档后，可在底部问答栏就当前项目提问。</p>
                </div>
              {/if}
            </div>
          {/if}

          <AskBar
            disabled={!project.selectedSlug || !project.acpOk}
            disabledReason={
              !project.selectedSlug
                ? "请先选择项目"
                : !project.acpOk
                  ? "请先在设置中配置 ACP 代理"
                  : null
            }
            placeholder={project.activeDoc
              ? `就「${project.activeHumanPath?.split("/").pop() ?? "当前文档"}」提问…`
              : "就当前项目提问…"}
            onclear={clearChatHistory}
            onask={handleAskInput}
          />
        </main>
      </div>
    {/if}
  </div>
</div>

{#if project.deepWikiOpen}
<DeepWikiPanel
  open={project.deepWikiOpen}
  projectSlug={project.selectedSlug}
  projectName={selectedProjectMetaDerived?.name ?? null}
  repoPath={project.selectedRepoPath}
  messages={currentMessages}
  initialQuestion={project.deepWikiInitialQuestion}
  sourceSlice={currentDeepWikiSource}
  onclose={closeDeepWiki}
  onmessageschange={(update) => {
    if (!project.selectedSlug) return;
    updateChat(project.selectedSlug, update);
  }}
  onsourcechange={(slice) => {
    if (!project.selectedSlug) return;
    setDeepWikiSource(project.selectedSlug, slice);
  }}
  onopenDoc={openDocPath}
/>
{/if}

{#if project.settingsOpen}
<SettingsPanel
  open={project.settingsOpen}
  onclose={() => (project.settingsOpen = false)}
  onsaved={async (status) => {
    project.llmStatus = status;
    try {
      const settings = await getModelSettings();
      project.agentExecution = normalizeAgentExecution(settings.acp?.agent_execution);
    } catch {
      // keep previous mode
    }
    project.acpOk = await checkAcp();
    const ok = hybridNativeLlm ? status.ready && project.acpOk : project.acpOk;
    setStatus(ok ? "设置已保存" : "请检查 ACP 与 LLM 配置", ok ? "success" : "error");
  }}
/>
{/if}

<SourceDrawer
  open={Boolean(knowledgeSourceSlice)}
  slice={knowledgeSourceSlice}
  repoPath={project.selectedRepoPath}
  onclose={() => project.selectedSlug && setKnowledgeSource(project.selectedSlug, null)}
  onSourceClick={openKnowledgeSourceCitation}
/>

{#if project.helpOpen}
<HelpPanel open={project.helpOpen} onclose={() => (project.helpOpen = false)} />
{/if}
