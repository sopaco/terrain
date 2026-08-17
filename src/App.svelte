<script lang="ts">
    import { onMount } from "svelte";
    import {
        Check,
        CircleX,
        Folder,
        PanelLeftOpen,
        Settings,
    } from "@lucide/svelte";
    import { open } from "@tauri-apps/plugin-dialog";
    import { listen } from "@tauri-apps/api/event";
    import AskBar from "./lib/components/AskBar.svelte";
    import AskCompletionNotice from "./lib/components/AskCompletionNotice.svelte";
    import HumanDocTree from "./lib/components/HumanDocTree.svelte";
    import MainNavTabs from "./lib/components/MainNavTabs.svelte";
    import SourceDrawer from "./lib/components/SourceDrawer.svelte";
    import ProjectOverviewPanel from "./lib/components/ProjectOverviewPanel.svelte";
    import ProjectSelector from "./lib/components/ProjectSelector.svelte";
    import HelpButton from "./lib/components/icons/HelpButton.svelte";
    import UsageMonitor from "./lib/components/UsageMonitor.svelte";
    import StatusBanner from "./lib/components/StatusBanner.svelte";
    import TaskProgressBar from "./lib/components/TaskProgressBar.svelte";
    import type { StatusKind } from "./lib/components/StatusBanner.svelte";
    import {
        bootstrapApp,
        checkAcp,
        computeFreshness,
        getKnowledgeRoot,
        getModelSettings,
        getProjectOverview,
        initializeProject,
        listHumanDocs,
        readProjectFreshnessCached,
        removeProject,
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
    import {
        usesNativeLlm,
        normalizeAgentExecution,
    } from "./lib/agentExecution";
    import { parseAskSlashCommand } from "./lib/askSlashCommands";
    import { loadAskProjectState, startNewAskSession, switchAskSession } from "./lib/askSession";
    import { scheduleIdle } from "./lib/scheduleIdle";
    import { initLocale, tr, t } from "./lib/i18n";
    import {
        citationToSourceSlice,
        createPendingSourceSlice,
    } from "./lib/resolveSource";
    import {
        setStatus,
        STATUS_AUTO_DISMISS_MS,
        status,
    } from "./lib/stores/status.svelte";
    import { readerLayout, toggleDocTree } from "./lib/stores/readerLayout.svelte";
    import {
        askStreamingBySlug,
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
        selectedRegistryProject,
        hasAnySelectableProject,
        setProjectTask,
    } from "./lib/stores/project.svelte";
    import type {
        ChatMessage,
        HumanDocEntry,
        KnowledgeDoc,
        ProjectRegistryEntry,
        SearchHit,
        SourceCitation,
        SourceSlice,
    } from "./lib/types";
    import {
        countRegistryByStatus,
        findRegistryProject,
        preferredRegistryProject,
        registryDisplayName,
        selectedProjectDisplayName,
    } from "./lib/projectRegistry";

    const hybridNativeLlm = $derived(usesNativeLlm(project.agentExecution));

    const selectedRegistryDerived = $derived(selectedRegistryProject());
    const selectedProjectLabel = $derived(
        selectedProjectDisplayName(
            project.selectedSlug,
            project.registryProjects,
            tr("terms.msg.selectProject"),
        ),
    );
    const currentTaskDerived = $derived(currentTask());
    const currentMessages = $derived(
        project.selectedSlug ? (chatSessions[project.selectedSlug] ?? []) : [],
    );
    const currentDeepWikiSource = $derived(
        project.selectedSlug
            ? (deepWikiSources[project.selectedSlug] ?? null)
            : null,
    );
    const knowledgeSourceSlice = $derived(
        project.selectedSlug
            ? (knowledgeSources[project.selectedSlug] ?? null)
            : null,
    );
    let knowledgeSourceLoadId = 0;
    const repackBusy = $derived(currentTaskDerived.repackBusy);
    const lithoBusy = $derived(currentTaskDerived.lithoBusy);
    const lithoProgress = $derived(currentTaskDerived.lithoProgress);

    type SddPanel = typeof import("./lib/components/SddWorkflowPanel.svelte").default;
    type EnvPanel = typeof import("./lib/components/EnvIntegratePanel.svelte").default;
    type DeepWikiPanelType = typeof import("./lib/components/DeepWikiPanel.svelte").default;
    type SettingsPanelType = typeof import("./lib/components/SettingsPanel.svelte").default;
    type HelpPanelType = typeof import("./lib/components/HelpPanel.svelte").default;
    type KnowledgeArticleType = typeof import("./lib/components/KnowledgeArticle.svelte").default;

    let SddWorkflowPanel = $state<SddPanel | null>(null);
    let EnvIntegratePanel = $state<EnvPanel | null>(null);
    let DeepWikiPanel = $state<DeepWikiPanelType | null>(null);
    let SettingsPanel = $state<SettingsPanelType | null>(null);
    let HelpPanel = $state<HelpPanelType | null>(null);
    let KnowledgeArticle = $state<KnowledgeArticleType | null>(null);

    let freshnessComputeSlug: string | null = null;

    $effect(() => {
        if (project.activeTab === "sdd" && !SddWorkflowPanel) {
            void import("./lib/components/SddWorkflowPanel.svelte").then((m) => {
                SddWorkflowPanel = m.default;
            });
        }
    });

    $effect(() => {
        if (project.activeTab === "env" && !EnvIntegratePanel) {
            void import("./lib/components/EnvIntegratePanel.svelte").then((m) => {
                EnvIntegratePanel = m.default;
            });
        }
    });

    const keepDeepWikiPanel = $derived(
        Boolean(
            project.selectedSlug &&
                (project.deepWikiOpen ||
                    Boolean(askStreamingBySlug[project.selectedSlug])),
        ),
    );

    $effect(() => {
        if (keepDeepWikiPanel && project.selectedSlug && !DeepWikiPanel) {
            void import("./lib/components/DeepWikiPanel.svelte").then((m) => {
                DeepWikiPanel = m.default;
            });
        }
    });

    $effect(() => {
        if (project.settingsOpen && !SettingsPanel) {
            void import("./lib/components/SettingsPanel.svelte").then((m) => {
                SettingsPanel = m.default;
            });
        }
    });

    $effect(() => {
        if (project.helpOpen && !HelpPanel) {
            void import("./lib/components/HelpPanel.svelte").then((m) => {
                HelpPanel = m.default;
            });
        }
    });

    $effect(() => {
        if (project.activeDoc && !KnowledgeArticle) {
            void import("./lib/components/KnowledgeArticle.svelte").then((m) => {
                KnowledgeArticle = m.default;
            });
        }
    });

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
        setStatus(t("app.status.refreshingProjects"), "loading");
        try {
            const boot = await bootstrapApp();
            initLocale(boot.model_settings.language);
            project.agentExecution = normalizeAgentExecution(
                boot.model_settings.acp?.agent_execution,
            );
            project.registryProjects = boot.registry_projects;
            project.llmStatus = boot.llm_status;
            project.acpOk = boot.acp_ok;

            const slug = project.selectedSlug;
            if (slug) {
                const entry = findRegistryProject(slug, project.registryProjects);
                if (entry) {
                    project.selectedRepoPath = entry.repo_path;
                    if (entry.status === "stale") {
                        project.projectOverview = null;
                        project.freshnessLoading = false;
                        await refreshKnowledgeRoot(slug);
                    } else {
                        await refreshKnowledgeRoot(slug);
                        if (project.activeTab === "overview") {
                            void loadProjectOverview(slug);
                        }
                    }
                } else {
                    project.selectedSlug = null;
                    project.selectedRepoPath = null;
                    project.projectOverview = null;
                    const fallback = preferredRegistryProject(
                        project.registryProjects,
                    );
                    if (fallback) {
                        selectRegistryProject(fallback);
                    } else {
                        await refreshKnowledgeRoot();
                    }
                }
            } else {
                const fallback = preferredRegistryProject(
                    project.registryProjects,
                );
                if (fallback) {
                    selectRegistryProject(fallback);
                } else {
                    await refreshKnowledgeRoot();
                }
            }

            const counts = countRegistryByStatus(project.registryProjects);
            const issues = counts.partial + counts.stale;
            const statusDetail =
                issues > 0
                    ? t("app.status.issuesDetail", { count: issues })
                    : "";
            setStatus(
                t("app.status.registeredProjects", {
                    count: project.registryProjects.length,
                    detail: statusDetail,
                }),
                "success",
            );
        } catch (e) {
            setStatus(String(e), "error");
        }
    }

    async function applyCachedFreshness(slug: string) {
        try {
            const cached = await readProjectFreshnessCached(slug);
            if (
                cached &&
                project.selectedSlug === slug &&
                project.projectOverview
            ) {
                project.projectOverview = mergeFreshnessIntoOverview(
                    project.projectOverview,
                    cached,
                );
            }
        } catch {
            /* keep overview without freshness */
        }
    }

    async function runFreshnessCompute(slug: string, repoPath: string) {
        const requestSlug = slug;
        project.freshnessLoading = true;
        try {
            const freshness = await computeFreshness(slug, repoPath);
            if (
                project.selectedSlug === requestSlug &&
                project.projectOverview
            ) {
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
    }

    function scheduleFreshnessCompute(slug: string, repoPath: string) {
        if (freshnessComputeSlug === slug) return;
        freshnessComputeSlug = slug;
        scheduleIdle(() => {
            if (project.selectedSlug !== slug) return;
            void runFreshnessCompute(slug, repoPath);
        });
    }

    function requestFreshnessCompute() {
        const slug = project.selectedSlug;
        const repoPath = project.projectOverview?.repo_path;
        if (!slug || !repoPath) return;
        if (project.freshnessLoading) return;
        if (project.projectOverview?.freshness?.overall_score != null) return;
        void runFreshnessCompute(slug, repoPath);
    }

    async function loadProjectOverviewFreshness(
        slug: string,
        repoPath: string | null | undefined,
    ) {
        if (!repoPath) {
            project.freshnessLoading = false;
            return;
        }
        freshnessComputeSlug = null;
        await applyCachedFreshness(slug);
        scheduleFreshnessCompute(slug, repoPath);
    }

    async function loadProjectOverview(
        slug: string,
        opts?: { skipFreshness?: boolean },
    ) {
        project.overviewLoading = true;
        try {
            project.projectOverview = await getProjectOverview(slug);
            if (project.projectOverview?.repo_path && !opts?.skipFreshness) {
                void loadProjectOverviewFreshness(
                    slug,
                    project.projectOverview.repo_path,
                );
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
            setStatus(t("app.status.pickFolderFailed", { error: String(e) }), "error");
            return;
        }
        if (!picked || Array.isArray(picked)) return;
        await triggerProjectInitialization(picked);
    }

    async function removeRegistryProject(entry: ProjectRegistryEntry) {
        project.pickerOpen = false;
        try {
            await removeProject(entry.slug);
            if (project.selectedSlug === entry.slug) {
                project.selectedSlug = null;
                project.selectedRepoPath = null;
                project.projectOverview = null;
                project.activeDoc = null;
                project.activeHumanPath = null;
                project.humanDocs = [];
                project.hits = [];
            }
            await refresh();
            setStatus(
                t("app.status.removedFromList", {
                    name: registryDisplayName(entry),
                }),
                "success",
            );
        } catch (e) {
            setStatus(String(e), "error");
        }
    }

    function selectRegistryProject(entry: ProjectRegistryEntry) {
        project.pickerOpen = false;
        if (project.selectedSlug !== entry.slug) {
            project.activeDoc = null;
            project.activeHumanPath = null;
            project.hits = [];
            project.deepWikiInitialQuestion = null;
        }
        project.selectedSlug = entry.slug;
        project.selectedRepoPath = entry.repo_path;
        project.activeTab = "overview";

        if (entry.status === "stale") {
            project.projectOverview = null;
            project.freshnessLoading = false;
            project.humanDocs = [];
            setStatus(
                t("app.status.projectNeedsRepair", {
                    name: registryDisplayName(entry),
                }),
                "idle",
                entry.slug,
                STATUS_AUTO_DISMISS_MS,
            );
            void refreshKnowledgeRoot(entry.slug);
            return;
        }

        setStatus(
            t("app.status.projectSelected", {
                name: registryDisplayName(entry),
            }),
            "idle",
            entry.slug,
            STATUS_AUTO_DISMISS_MS,
        );
        void loadHumanDocs(entry.slug);
        void loadProjectOverview(entry.slug);
        void refreshKnowledgeRoot(entry.slug);
        void loadAskProjectState(entry.slug);
    }

    async function openFolderPath(path: string) {
        try {
            await openRepoFolder(path);
        } catch (e) {
            setStatus(t("terms.msg.openFolderFailed", { error: String(e) }), "error");
        }
    }

    function parseProgressLabel(label: string): {
        stage: string | null;
        message: string;
    } {
        const match = label.match(/^\[([^\]]+)\]\s*(.*)$/);
        if (!match) return { stage: null, message: label };
        return { stage: match[1], message: match[2] || label };
    }

    const lithoProgressParts = $derived(parseProgressLabel(lithoProgress));

    const showTaskProgressBar = $derived(
        Boolean(
            project.selectedSlug &&
            (lithoProgress || (project.initBusy && project.initProgress)),
        ),
    );

    const showStatusBar = $derived(
        !showTaskProgressBar &&
            (status.kind !== "idle" ||
                status.message !== tr("terms.statusChip.idle")),
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

    async function triggerProjectInitialization(
        repoPath: string,
        slug?: string,
    ) {
        if (project.initBusy) return;
        project.initBusy = true;
        project.initProgress = t("app.status.scanningRepo");
        const targetSlug = slug ?? null;
        if (targetSlug) {
            project.selectedSlug = targetSlug;
            project.selectedRepoPath = repoPath;
            setProjectTask(targetSlug, {
                repackBusy: true,
                lithoBusy: false,
                lithoProgress: "",
            });
        }
        try {
            const result = await initializeProject(repoPath, slug);
            project.selectedSlug = result.project_slug;
            project.selectedRepoPath = result.repo_path;
            const note = result.notes.length
                ? ` · ${result.notes.join("；")}`
                : "";
            const lithoNote =
                result.litho_ran && !result.human_docs_complete
                    ? ` · ${t("app.status.lithoIncomplete")}`
                    : "";
            setStatus(
                t("app.status.initComplete", {
                    scan: result.scan_files_written,
                    term: t("terms.humanKnowledge"),
                    count: result.human_doc_count,
                    lithoNote,
                    note,
                }),
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
                setProjectTask(targetSlug, {
                    repackBusy: false,
                    lithoBusy: false,
                    lithoProgress: "",
                });
            } else if (project.selectedSlug) {
                setProjectTask(project.selectedSlug, {
                    repackBusy: false,
                    lithoBusy: false,
                    lithoProgress: "",
                });
            }
        }
    }

    async function triggerQuickRefresh() {
        if (!project.selectedRepoPath || !project.selectedSlug) {
            setStatus(t("terms.msg.selectProjectWithRepo"), "error");
            return;
        }
        if (project.quickRefreshBusy) return;
        const slug = project.selectedSlug;
        project.quickRefreshBusy = true;
        setStatus(
            t("app.status.quickRefreshing"),
            "progress",
            slug,
        );
        try {
            const result = await runQuickRefresh(
                project.selectedRepoPath,
                slug,
            );
            const note = result.notes.length
                ? ` · ${result.notes.join("；")}`
                : "";
            setStatus(
                t("app.status.refreshDone", {
                    score: result.freshness.overall_score,
                    note,
                }),
                result.freshness.overall_stale ? "idle" : "success",
                slug,
            );
            await loadProjectOverview(slug, { skipFreshness: true });
            if (project.selectedSlug === slug && project.projectOverview) {
                project.projectOverview = mergeFreshnessIntoOverview(
                    project.projectOverview,
                    result.freshness,
                );
            }
            project.freshnessLoading = false;
        } catch (e) {
            setStatus(String(e), "error");
        } finally {
            project.quickRefreshBusy = false;
            // Quick refresh may emit litho-progress (incremental human-doc update) without a
            // litho-done event, so clear the label here rather than leaving it stuck.
            setProjectTask(slug, { lithoProgress: "" });
        }
    }

    async function triggerAgentContextGeneration() {
        if (!project.selectedRepoPath || !project.selectedSlug) {
            setStatus(t("terms.msg.selectProjectWithRepo"), "error");
            return;
        }
        if (project.agentContextBusy) return;
        if (!project.acpOk) {
            setStatus(t("app.configureAcp"), "error");
            return;
        }
        if (hybridNativeLlm && !project.llmStatus?.ready) {
            setStatus(t("app.configureLlm"), "error");
            return;
        }
        const slug = project.selectedSlug;
        project.agentContextBusy = true;
        setStatus(t("terms.msg.agentContextGenerating"), "progress", slug);
        try {
            // Overview 「生成」/「重新生成」 are explicit rebuild requests — never incremental.
            await runAgentContextGeneration(project.selectedRepoPath, slug, true);
            setStatus(t("terms.msg.agentContextReady"), "success");
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
            // Overview doc is `1.概述.md` (zh) or `1.Overview.md` (en) depending
            // on the generation language.
            const overview = docs.find((d) =>
                /^1\..*\.md$/i.test(d.relative_path),
            );
            if (overview) {
                project.activeTab = "knowledge";
                await openHumanDoc(overview);
            } else {
                setStatus(
                    t("app.status.noOverviewDoc", {
                        term: t("terms.humanKnowledge"),
                    }),
                    "error",
                );
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
                    (d) =>
                        d.section === "agent" &&
                        d.relative_path === "agent/meta-inputs.md",
                );
                const structured = docs.filter(
                    (d) => d.section === "structured",
                );
                const target = agentMeta ?? structured[0];
                if (target) {
                    await openHumanDoc(target);
                } else {
                    setStatus(
                        t("app.status.noStructuredEntries", {
                            term: t("terms.agentKnowledge"),
                        }),
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
        setStatus(t("app.status.openingDoc", { title: doc.title }), "loading");
        try {
            project.activeDoc = await readDocument(doc.path);
            setStatus(t("app.status.viewingDoc", { title: doc.title }), "idle");
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
        setStatus(t("app.status.rebuildingIndex"), "progress", project.selectedSlug);
        try {
            const pack = await packAgentAssets(
                project.selectedRepoPath,
                project.selectedSlug,
            );
            setStatus(
                t("app.status.indexUpdated", {
                    files: pack.total_files,
                    tokens: pack.total_tokens,
                }),
                "success",
            );
            if (project.selectedSlug)
                await loadProjectOverview(project.selectedSlug);
        } catch (e) {
            setStatus(String(e), "error");
        } finally {
            if (project.selectedSlug)
                setProjectTask(project.selectedSlug, { repackBusy: false });
        }
    }

    async function triggerHumanGeneration(forceRefresh?: boolean) {
        if (!project.selectedRepoPath || !project.selectedSlug) {
            setStatus(t("terms.msg.selectProjectWithRepoPath"), "error");
            return;
        }
        if (!project.acpOk) {
            setStatus(t("app.status.acpNotFound"), "error");
            return;
        }
        const slug = project.selectedSlug;
        const force =
            typeof forceRefresh === "boolean"
                ? forceRefresh
                : (project.projectOverview?.litho.human_docs_complete ??
                  false);
        setProjectTask(slug, {
            lithoBusy: true,
            lithoProgress: force
                ? t("app.status.regeneratingHuman", {
                      term: t("terms.humanKnowledge"),
                  })
                : t("app.status.generatingHuman", {
                      term: t("terms.humanKnowledge"),
                  }),
        });
        try {
            await runLithoGeneration(project.selectedRepoPath, slug, force);
        } catch (e) {
            setStatus(String(e), "error");
            setProjectTask(slug, { lithoBusy: false, lithoProgress: "" });
        }
    }

    async function openKnowledgeSourceCitation(c: SourceCitation) {
        if (!project.selectedSlug) return;

        const slug = project.selectedSlug;
        const loadId = ++knowledgeSourceLoadId;
        setKnowledgeSource(
            slug,
            createPendingSourceSlice(c, project.selectedRepoPath),
        );

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

    async function startNewAskChat() {
        if (!project.selectedSlug) return;
        try {
            await startNewAskSession(project.selectedSlug);
            setStatus(t("app.status.newChatCreated"), "success");
        } catch (e) {
            setStatus(String(e), "error");
        }
    }

    function handleAskInput(q: string) {
        if (!project.selectedSlug) {
            setStatus(t("app.selectProjectFirst"), "error");
            return;
        }
        if (parseAskSlashCommand(q)?.type === "new") {
            void startNewAskChat();
            return;
        }
        openDeepWiki(q);
    }

    function openDeepWiki(question?: string) {
        if (!project.selectedSlug) {
            setStatus(t("app.selectProjectFirst"), "error");
            return;
        }
        if (question && parseAskSlashCommand(question)?.type === "new") {
            void startNewAskChat();
            project.deepWikiOpen = true;
            project.deepWikiInitialQuestion = null;
            return;
        }
        if (!project.acpOk) {
            setStatus(t("app.configureAcp"), "error");
            return;
        }
        project.deepWikiInitialQuestion = question ?? null;
        project.deepWikiOpen = true;
    }

    function closeDeepWiki() {
        project.deepWikiOpen = false;
        project.deepWikiInitialQuestion = null;
    }

    async function openDeepWikiFromNotice(sessionId: string) {
        if (!project.selectedSlug) return;
        project.deepWikiOpen = true;
        project.deepWikiInitialQuestion = null;
        if (sessionId) {
            try {
                await switchAskSession(
                    project.selectedSlug,
                    sessionId,
                    currentMessages,
                );
            } catch (e) {
                setStatus(String(e), "error");
            }
        }
    }

    async function runSearch() {
        const q = project.query.trim();
        if (!q) {
            project.hits = [];
            return;
        }
        project.docLoading = true;
        setStatus(t("app.status.searchingFor", { query: q }), "loading");
        try {
            project.hits = await searchKnowledge(
                q,
                project.selectedSlug ?? undefined,
            );
            project.activeDoc = null;
            setStatus(
                `${project.hits.length} result(s)`,
                project.hits.length ? "success" : "idle",
            );
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
            requestAnimationFrame(() =>
                document.getElementById("search-input")?.focus(),
            );
        }
        if (
            e.key === "Enter" &&
            document.activeElement?.id === "search-input"
        ) {
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
            unlistenInitProgress = await listen<{
                project_slug: string;
                stage: string;
                message: string;
            }>("project-init-progress", (ev) => {
                const { project_slug, message } = ev.payload;
                project.initProgress = message;
                if (ev.payload.stage === "human_docs") {
                    setProjectTask(project_slug, {
                        lithoBusy: true,
                        lithoProgress: message,
                    });
                } else if (ev.payload.stage === "scan") {
                    setProjectTask(project_slug, { repackBusy: true });
                } else if (ev.payload.stage === "done") {
                    setProjectTask(project_slug, {
                        lithoBusy: false,
                        lithoProgress: "",
                        repackBusy: false,
                    });
                }
            });
            unlistenProgress = await listen<{
                project_slug: string;
                stage: string;
                message: string;
            }>("litho-progress", (ev) => {
                const { project_slug, stage, message } = ev.payload;
                const label = `[${stage}] ${message}`;
                setProjectTask(project_slug, { lithoProgress: label });
            });
            unlistenDone = await listen<{
                project_slug: string;
                result: {
                    human_doc_count: number;
                    human_docs_complete: boolean;
                };
            }>("litho-done", async (ev) => {
                const { project_slug, result } = ev.payload;
                setProjectTask(project_slug, {
                    lithoBusy: false,
                    lithoProgress: "",
                });
                const count = result.human_doc_count;
                const complete = result.human_docs_complete;
                const msg = !complete
                    ? count === 0
                        ? t("app.status.lithoDoneNoDocs", {
                              term: t("terms.humanKnowledge"),
                              slug: project_slug,
                          })
                        : t("app.status.lithoIncompleteWithCount", {
                              term: t("terms.humanKnowledge"),
                              slug: project_slug,
                              count,
                          })
                    : t("app.status.lithoReady", {
                          term: t("terms.humanKnowledge"),
                          slug: project_slug,
                          count,
                      });
                if (project.selectedSlug === project_slug) {
                    setStatus(
                        msg,
                        complete ? "success" : count === 0 ? "error" : "idle",
                    );
                    await loadHumanDocs(project_slug);
                    await loadProjectOverview(project_slug);
                }
            });
        })();

        return () => {
            window.removeEventListener("keydown", onKeydown);
            unlistenProgress?.();
            unlistenDone?.();
            unlistenInitProgress?.();
        };
    });
</script>

<div class="flex h-screen">
    <aside
        class="flex w-16 shrink-0 flex-col items-center gap-2 border-r border-tr-border bg-tr-surface py-3"
    >
        <div
            class="mb-1 flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br from-tr-accent to-tr-page text-tr-on-accent"
            title="Terrain"
            aria-hidden="true"
        >
            <svg
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.6"
                class="h-[18px] w-[18px]"
                ><path d="M3 19h18L15 6l-3.5 6.5L9 9 3 19z" /></svg
            >
        </div>

        <MainNavTabs
            active={project.activeTab}
            disabled={!project.selectedSlug && !hasAnySelectableProject()}
            onchange={(tab) => {
                project.activeTab = tab;
                if (tab === "knowledge" && !project.selectedSlug) {
                    const fallback = preferredRegistryProject(
                        project.registryProjects,
                    );
                    if (fallback) {
                        selectRegistryProject(fallback);
                    }
                }
            }}
        />

        <div class="flex-1"></div>

        <div
            class="flex flex-col items-center gap-1.5 border-t border-tr-border pt-2"
        >
            <button
                type="button"
                class="tr-press inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border border-tr-border-strong text-tr-ink-2 transition-colors hover:bg-tr-elevated"
                title={tr("app.settings")}
                aria-label={tr("app.settings")}
                onclick={() => (project.settingsOpen = true)}
            >
                <Settings size={16} strokeWidth={2} aria-hidden="true" />
            </button>
            <HelpButton
                onclick={() => (project.helpOpen = true)}
                title={tr("app.glossary")}
                ariaLabel={tr("app.glossary")}
                variant="toolbar"
                class="!h-9 !w-9"
            />
        </div>
    </aside>

    <div class="flex min-h-0 flex-1 flex-col">
        <header
            class="flex shrink-0 items-center gap-3 overflow-x-auto border-b border-tr-border bg-tr-page px-4 py-2"
        >
            <ProjectSelector
                registryProjects={project.registryProjects}
                selectedSlug={project.selectedSlug}
                open={project.pickerOpen}
                addBusy={project.initBusy}
                ontoggle={() => (project.pickerOpen = !project.pickerOpen)}
                onselect={selectRegistryProject}
                onadd={addProject}
                onremove={removeRegistryProject}
                onopenFolder={(entry) => openFolderPath(entry.repo_path)}
            />

            <div class="ml-auto flex shrink-0 items-center gap-2">
                {#if showStatusBar}
                    <StatusBanner
                        message={status.message}
                        kind={status.kind}
                        detail={status.detail}
                    />
                {/if}
                <UsageMonitor />
            </div>
        </header>

        {#if showTaskProgressBar && project.selectedSlug}
            <TaskProgressBar
                projectSlug={project.selectedSlug}
                stage={project.initBusy ? tr("app.initializing") : lithoProgressParts.stage}
                message={project.initBusy && project.initProgress
                    ? project.initProgress
                    : lithoProgressParts.message}
            />
        {/if}

        <div
            class="flex min-h-0 flex-1 flex-col"
            class:hidden={project.activeTab !== "overview"}
        >
            <ProjectOverviewPanel
            overview={project.projectOverview}
            loading={project.overviewLoading}
            acpOk={project.acpOk}
            llmReady={project.llmStatus?.ready ?? false}
            {hybridNativeLlm}
            agentContextBusy={project.agentContextBusy}
            {lithoBusy}
            {repackBusy}
            initBusy={project.initBusy}
            initProgress={project.initProgress}
            registryProjects={project.registryProjects}
            selectedRegistry={selectedRegistryDerived}
            onOpenKnowledge={() => (project.activeTab = "knowledge")}
            onOpenEnv={() => (project.activeTab = "env")}
            onOpenSettings={() => (project.settingsOpen = true)}
            onOpenAsk={() => openDeepWiki()}
            onGenerateHuman={() => void triggerHumanGeneration()}
            onGenerateAgentContext={triggerAgentContextGeneration}
            onRepack={packAgentForSelected}
            onInitializeProject={triggerProjectInitialization}
            onOpenPath={openFolderPath}
            onOpenArchitectureDoc={project.projectOverview?.agent_context.ready
                ? openArchitectureDoc
                : undefined}
            onOpenHumanOverview={project.projectOverview?.litho.has_human_docs
                ? openOverviewHumanDoc
                : undefined}
            onOpenStructured={openStructuredDocs}
            quickRefreshBusy={project.quickRefreshBusy}
            freshnessLoading={project.freshnessLoading}
            onQuickRefresh={triggerQuickRefresh}
            onRequestFreshnessCompute={requestFreshnessCompute}
            onSaveProjectRemark={async (remark) => {
                if (!project.selectedSlug) return;
                const prevFreshness = project.projectOverview?.freshness;
                const updated = await saveProjectRemark(
                    project.selectedSlug,
                    remark,
                );
                project.projectOverview =
                    prevFreshness && !updated.freshness
                        ? mergeFreshnessIntoOverview(updated, prevFreshness)
                        : updated;
            }}
        />
    </div>

    <div
        class="flex min-h-0 flex-1 flex-col"
        class:hidden={project.activeTab !== "sdd"}
    >
        {#if project.activeTab === "sdd" && SddWorkflowPanel}
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
            {#if EnvIntegratePanel}
                <EnvIntegratePanel
                    repoPath={project.selectedRepoPath}
                    onStatus={(message, kind) => setStatus(message, kind)}
                    onIntegrated={() => {
                        if (project.selectedSlug)
                            void loadProjectOverview(project.selectedSlug);
                    }}
                />
            {/if}
        </div>
    {/if}

    <div
        class="flex min-h-0 flex-1 flex-col"
        class:hidden={project.activeTab !== "knowledge"}
    >
        {#if project.activeTab === "knowledge"}
            <div
                class="flex shrink-0 items-center gap-2 border-b border-tr-border-strong bg-tr-surface/80 px-4 py-2"
            >
                <input
                    id="search-input"
                    class="min-w-0 flex-1 rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-1.5 text-sm outline-none focus:border-tr-accent"
                    placeholder={tr("app.searchPlaceholder", { term: tr("terms.knowledgeTab") })}
                    bind:value={project.query}
                />
                <button
                    type="button"
                    class="tr-press shrink-0 rounded-lg bg-tr-elevated px-3 py-1.5 text-sm transition-colors hover:bg-tr-raised disabled:opacity-50"
                    disabled={project.docLoading}
                    onclick={runSearch}
                >
                    {tr("common.search")}
                </button>
                {#if project.selectedSlug}
                    <div
                        class="flex shrink-0 items-center gap-2 border-l border-tr-border-strong pl-3"
                    >
                        <button
                            type="button"
                            class="tr-press rounded-lg border border-tr-border-strong px-2.5 py-1.5 text-xs transition-colors hover:bg-tr-elevated disabled:opacity-50"
                            disabled={repackBusy ||
                                lithoBusy ||
                                !project.selectedRepoPath ||
                                !project.acpOk}
                            onclick={() => void triggerHumanGeneration()}
                            title={!project.acpOk
                                ? tr("app.configureAcpTitle")
                                : undefined}
                        >
                            {lithoBusy
                                ? tr("terms.generating")
                                : tr("terms.generate", {
                                      term: tr("terms.humanKnowledge"),
                                  })}
                        </button>
                    </div>
                {/if}
            </div>

            <div class="flex min-h-0 flex-1">
                {#if readerLayout.docTreeCollapsed}
                    <aside
                        class="flex w-9 shrink-0 flex-col items-center gap-3 border-r border-tr-border-strong bg-tr-surface py-2.5"
                    >
                        <button
                            type="button"
                            class="tr-press inline-flex shrink-0 items-center justify-center rounded-md p-1 text-tr-ink-3 transition-colors hover:bg-tr-elevated hover:text-tr-ink"
                            onclick={toggleDocTree}
                            aria-label={tr("app.expandDocTree")}
                            title={tr("app.expandDocTree")}
                        >
                            <PanelLeftOpen
                                size={14}
                                strokeWidth={2}
                                aria-hidden="true"
                            />
                        </button>
                        <span
                            class="select-none text-[10px] tracking-wider text-tr-ink-3 [writing-mode:vertical-rl]"
                            >{tr("app.docTree")}</span
                        >
                    </aside>
                {:else}
                <aside
                    class="flex w-60 shrink-0 flex-col border-r border-tr-border-strong bg-tr-surface"
                >
                    <HumanDocTree
                        docs={project.humanDocs}
                        activePath={project.activeHumanPath}
                        loading={project.humanDocsLoading}
                        onselect={openHumanDoc}
                        oncollapse={toggleDocTree}
                    />
                    <div
                        class="mt-auto border-t border-tr-border-strong px-3 py-2 text-[10px] text-tr-ink-3"
                    >
                        <div
                            class="flex items-center gap-1 truncate"
                            title={project.knowledgeRoot ||
                                project.selectedRepoPath ||
                                "—"}
                        >
                            <Folder
                                size={10}
                                strokeWidth={2}
                                class="shrink-0 text-tr-ink-3"
                                aria-hidden="true"
                            />
                            <span class="truncate">
                                {project.knowledgeRoot ||
                                    (project.selectedRepoPath
                                        ? `${project.selectedRepoPath}/.terrain`
                                        : "—")}
                            </span>
                        </div>
                        <div
                            class="mt-1 flex flex-wrap items-center gap-x-2 gap-y-0.5"
                        >
                            <span class="inline-flex items-center gap-1">
                                ACP
                                {#if project.acpOk}
                                    <Check
                                        size={10}
                                        strokeWidth={2.5}
                                        class="text-tr-good"
                                        aria-hidden="true"
                                    />
                                {:else}
                                    <CircleX
                                        size={10}
                                        strokeWidth={2.5}
                                        class="text-tr-critical"
                                        aria-hidden="true"
                                    />
                                {/if}
                            </span>
                            {#if hybridNativeLlm}
                                <span class="inline-flex items-center gap-1">
                                    LLM
                                    {#if project.llmStatus?.ready}
                                        <Check
                                            size={10}
                                            strokeWidth={2.5}
                                            class="text-tr-good"
                                            aria-hidden="true"
                                        />
                                    {:else}
                                        <CircleX
                                            size={10}
                                            strokeWidth={2.5}
                                            class="text-tr-critical"
                                            aria-hidden="true"
                                        />
                                    {/if}
                                </span>
                            {/if}
                        </div>
                    </div>
                </aside>
                {/if}

                <main class="flex min-w-0 flex-1 flex-col">
                    {#if project.docLoading}
                        <div
                            class="flex flex-1 flex-col items-center justify-center gap-3 text-sm text-tr-ink-3"
                        >
                            <span
                                class="inline-block h-8 w-8 animate-spin rounded-full border-2 border-tr-accent border-t-transparent"
                            ></span>
                            <span>{tr("terms.msg.loadingDocument")}</span>
                        </div>
                    {:else if project.activeDoc && KnowledgeArticle}
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
                                                class="mb-2 w-full rounded-lg border border-tr-border-strong bg-tr-elevated px-4 py-3 text-left transition-colors hover:bg-tr-raised"
                                                onclick={() => openHit(hit)}
                                            >
                                                <div
                                                    class="flex items-center gap-2"
                                                >
                                                    <span
                                                        class="rounded bg-tr-elevated px-1.5 py-0.5 text-[10px] uppercase"
                                                        >{hit.doc_type}</span
                                                    >
                                                    <span class="font-medium"
                                                        >{hit.title ??
                                                            hit.path}</span
                                                    >
                                                </div>
                                                <p
                                                    class="mt-1 text-sm text-tr-ink-3"
                                                >
                                                    {hit.snippet}
                                                </p>
                                            </button>
                                        </li>
                                    {/each}
                                </ul>
                            {:else}
                                <div
                                    class="flex h-full flex-col items-center justify-center gap-2 px-6 text-center text-tr-ink-3"
                                >
                                    <p class="text-lg text-tr-ink-2">
                                        {project.selectedSlug
                                            ? tr("app.selectDocFromTree")
                                            : tr("app.addOrSelectProject")}
                                    </p>
                                    <p class="text-sm">
                                        {tr("app.askHint")}
                                    </p>
                                </div>
                            {/if}
                        </div>
                    {/if}

                    <AskBar
                        disabled={!project.selectedSlug || !project.acpOk}
                        disabledReason={!project.selectedSlug
                            ? tr("app.selectProjectFirst")
                            : !project.acpOk
                              ? tr("app.configureAcpTitle")
                              : null}
                        placeholder={project.activeDoc
                            ? tr("app.askDocPlaceholder", {
                                  doc:
                                      project.activeHumanPath
                                          ?.split("/")
                                          .pop() ?? tr("app.currentDoc"),
                              })
                            : tr("app.askProjectPlaceholder")}
                        onclear={() => void startNewAskChat()}
                        onopen={() => openDeepWiki()}
                        onask={handleAskInput}
                    />
                </main>
            </div>
        {/if}
    </div>
    </div>
</div>

{#if project.selectedSlug && DeepWikiPanel && keepDeepWikiPanel}
    <DeepWikiPanel
        open={project.deepWikiOpen}
        projectSlug={project.selectedSlug}
        projectName={selectedProjectLabel}
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

<AskCompletionNotice
    repoPath={project.selectedRepoPath}
    onOpenAsk={openDeepWikiFromNotice}
/>

{#if project.settingsOpen && SettingsPanel}
    <SettingsPanel
        open={project.settingsOpen}
        onclose={() => (project.settingsOpen = false)}
        onsaved={async (status) => {
            project.llmStatus = status;
            try {
                const settings = await getModelSettings();
                project.agentExecution = normalizeAgentExecution(
                    settings.acp?.agent_execution,
                );
            } catch {
                // keep previous mode
            }
            project.acpOk = await checkAcp();
            const ok = hybridNativeLlm
                ? status.ready && project.acpOk
                : project.acpOk;
            setStatus(
                ok ? tr("app.settingsSaved") : tr("app.checkAcpLlmConfig"),
                ok ? "success" : "error",
            );
        }}
    />
{/if}

<SourceDrawer
    open={Boolean(knowledgeSourceSlice)}
    slice={knowledgeSourceSlice}
    repoPath={project.selectedRepoPath}
    onclose={() =>
        project.selectedSlug && setKnowledgeSource(project.selectedSlug, null)}
    onSourceClick={openKnowledgeSourceCitation}
/>

{#if project.helpOpen && HelpPanel}
    <HelpPanel
        open={project.helpOpen}
        onclose={() => (project.helpOpen = false)}
    />
{/if}
