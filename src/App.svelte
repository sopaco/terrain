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
    import type { ProgressStep } from "./lib/components/TaskProgressBar.svelte";
    import type { StatusKind } from "./lib/components/StatusBanner.svelte";
    import {
        checkAcp,
        computeFreshness,
        getKnowledgeRoot,
        getModelSettings,
        getProjectOverview,
        initializeProject,
        listHumanDocs,
        listRegistryProjects,
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
    import { loadAppBootstrap } from "./lib/appBootstrap";
    import { findHumanOverviewDoc } from "./lib/humanDocPath";
    import { mergeFreshnessIntoOverview } from "./lib/mergeFreshness";
    import {
        usesNativeLlm,
        normalizeAgentExecution,
    } from "./lib/agentExecution";
    import { parseAskSlashCommand } from "./lib/askSlashCommands";
    import { loadAskProjectState, startNewAskSession, switchAskSession } from "./lib/askSession";
    import { scheduleIdle } from "./lib/scheduleIdle";
    import { applyLocale, isIdleStatusMessage, tr, t } from "./lib/i18n";
    import {
        citationToSourceSlice,
        createPendingSourceSlice,
    } from "./lib/resolveSource";
    import {
        setErrorStatus,
        setLocalizedErrorStatus,
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
        applyImportProgress,
        backgroundLithoTask,
        beginImport,
        clearProjectTask,
        currentTask,
        finishImport,
        isKnownProjectSlug,
        project,
        selectedRegistryProject,
        hasAnySelectableProject,
        setProjectTask,
        updateImportJob,
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
        importDisplayName,
        preferredRegistryProject,
        registryDisplayName,
        registryDisplayNameForSlug,
        selectedProjectDisplayName,
    } from "./lib/projectRegistry";

    const hybridNativeLlm = $derived(usesNativeLlm(project.agentExecution));

    // Ask runs through the external ACP agent in pure ACP mode and through the native
    // LLM in hybrid mode, so a missing ACP binary does not block Ask in hybrid mode.
    const askUnavailable = $derived(
        hybridNativeLlm ? !project.llmStatus?.ready : !project.acpOk,
    );

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
    // Per-view request tokens: a slow reply for project A must never paint over
    // project B after the user switched. Kept per slug so a background request
    // for another project cannot invalidate the visible one.
    let docLoadSeq = 0;
    const overviewLoadSeq = new Map<string, number>();
    const humanDocsLoadSeq = new Map<string, number>();

    function nextLoadSeq(seq: Map<string, number>, slug: string): number {
        const next = (seq.get(slug) ?? 0) + 1;
        seq.set(slug, next);
        return next;
    }
    const repackBusy = $derived(currentTaskDerived.repackBusy);
    const lithoBusy = $derived(currentTaskDerived.lithoBusy);
    const agentContextBusy = $derived(currentTaskDerived.agentContextBusy);
    const quickRefreshBusy = $derived(currentTaskDerived.quickRefreshBusy);
    const freshnessLoading = $derived(
        project.freshnessLoadingSlug === project.selectedSlug,
    );

    // ── Import / background task model ──────────────────────────────────────
    // The import target is its own entity: a freshly picked repository has no
    // slug and is not selected, so the progress UI must never borrow the
    // selected project's identity.
    const importBusy = $derived(Boolean(project.importJob));
    const importTargetSlug = $derived(project.importJob?.slug ?? null);
    const importTargetRepoPath = $derived(project.importJob?.repoPath ?? null);
    const importProgressMessage = $derived(project.importJob?.message ?? null);
    const importProjectName = $derived(
        project.importJob
            ? importDisplayName(
                  project.importJob.slug,
                  project.importJob.repoPath,
                  project.registryProjects,
              )
            : "",
    );

    const IMPORT_STAGES = ["scan", "human_docs", "agent_context"] as const;

    function importStageIndex(stage: string | null): number {
        if (stage === "done") return IMPORT_STAGES.length;
        const idx = IMPORT_STAGES.indexOf(
            stage as (typeof IMPORT_STAGES)[number],
        );
        return idx < 0 ? 0 : idx;
    }

    function importStageLabel(stage: string | null): string | null {
        switch (stage) {
            case "scan":
                return tr("misc.progress.stageScan");
            case "human_docs":
                return tr("misc.progress.stageHumanDocs", {
                    term: tr("terms.humanKnowledge"),
                });
            case "agent_context":
                return tr("misc.progress.stageAgentContext", {
                    term: tr("terms.agentKnowledge"),
                });
            case "done":
                return tr("misc.progress.stageDone");
            default:
                return null;
        }
    }

    function buildImportSteps(
        stage: string | null,
        seen: string[],
    ): ProgressStep[] {
        const idx = importStageIndex(stage);
        const skippedTitle = tr("misc.progress.stepSkippedTitle");
        return IMPORT_STAGES.map((key, i) => {
            const ran = seen.includes(key);
            const passed = stage === "done" || i < idx;
            const state = passed
                ? ran
                    ? ("done" as const)
                    : ("skipped" as const)
                : i === idx
                  ? ("active" as const)
                  : ("pending" as const);
            return {
                key,
                label: importStageLabel(key) ?? key,
                state,
                title: state === "skipped" ? skippedTitle : undefined,
            };
        });
    }

    function importPercent(stage: string | null): number {
        if (stage === "done") return 100;
        return [15, 55, 85][Math.min(importStageIndex(stage), 2)] ?? 15;
    }

    /** Human label for a slug, including the repo of an import not yet registered. */
    function projectLabelForSlug(slug: string): string {
        const job = project.importJob;
        if (job && job.slug === slug) {
            return importDisplayName(
                slug,
                job.repoPath,
                project.registryProjects,
            );
        }
        return registryDisplayNameForSlug(slug, project.registryProjects);
    }

    type TaskBarModel = {
        kind: "import" | "litho" | "background";
        slug: string | null;
        repoPath: string | null;
        title: string;
        projectName: string;
        stage: string | null;
        message: string;
        steps: ProgressStep[];
        percent: number | null;
        note: string | null;
        onswitch: (() => void) | null;
    };

    const taskBar = $derived.by((): TaskBarModel | null => {
        const job = project.importJob;
        if (job) {
            const name = importDisplayName(
                job.slug,
                job.repoPath,
                project.registryProjects,
            );
            const focused =
                job.slug != null && job.slug === project.selectedSlug;
            let note: string | null = null;
            if (!focused) {
                note =
                    job.selectionAtStart === project.selectedSlug
                        ? tr("misc.progress.importWillFocus", { name })
                        : tr("misc.progress.importNoFocus", { name });
            }
            return {
                kind: "import",
                slug: job.slug,
                repoPath: job.repoPath,
                title: tr("misc.progress.importTitle"),
                projectName: name,
                stage: importStageLabel(job.stage),
                message: job.message ?? tr("app.status.scanningRepo"),
                steps: buildImportSteps(job.stage, job.seenStages),
                percent: importPercent(job.stage),
                note,
                onswitch: null,
            };
        }

        const selectedSlug = project.selectedSlug;
        if (selectedSlug && currentTaskDerived.lithoProgress) {
            const parsed = parseProgressLabel(
                currentTaskDerived.lithoProgress,
            );
            return {
                kind: "litho",
                slug: selectedSlug,
                repoPath: project.selectedRepoPath,
                title: tr("misc.progress.generating"),
                projectName: selectedProjectLabel,
                stage: parsed.stage,
                message: parsed.message,
                steps: [],
                percent: null,
                note: null,
                onswitch: null,
            };
        }

        const background = backgroundLithoTask();
        if (background) {
            const parsed = parseProgressLabel(background.progress);
            const name = registryDisplayNameForSlug(
                background.slug,
                project.registryProjects,
            );
            return {
                kind: "background",
                slug: background.slug,
                repoPath: null,
                title: tr("misc.progress.backgroundTitle"),
                projectName: name,
                stage: parsed.stage,
                message: parsed.message,
                steps: [],
                percent: null,
                note: tr("misc.progress.backgroundNote", { name }),
                onswitch: () => focusProjectBySlug(background.slug),
            };
        }

        return null;
    });

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
        // Never paint a path that belongs to a project the user already left.
        if (project.selectedSlug !== target) return;
        try {
            const root = await getKnowledgeRoot(target);
            if (project.selectedSlug === target) project.knowledgeRoot = root;
        } catch {
            if (project.selectedSlug === target) project.knowledgeRoot = "";
        }
    }

    async function refresh(options?: { force?: boolean }) {
        setStatus(t("app.status.refreshingProjects"), "loading");
        const selectionAtRequest = project.selectedSlug;
        try {
            const boot = await loadAppBootstrap(options);
            applyLocale(boot.model_settings.language);
            project.agentExecution = normalizeAgentExecution(
                boot.model_settings.acp?.agent_execution,
            );
            project.registryProjects = boot.registry_projects;
            project.llmStatus = boot.llm_status;
            project.acpOk = boot.acp_ok;

            const slug = project.selectedSlug;
            // A selection made while this refresh was in flight is newer than the
            // snapshot we just received — never discard it.
            const selectionChangedDuringLoad =
                slug !== selectionAtRequest;
            if (slug) {
                const entry = findRegistryProject(slug, project.registryProjects);
                if (entry) {
                    project.selectedRepoPath = entry.repo_path;
                    if (entry.status === "stale") {
                        project.projectOverview = null;
                        project.freshnessLoadingSlug = null;
                        await refreshKnowledgeRoot(slug);
                    } else {
                        await refreshKnowledgeRoot(slug);
                        if (project.activeTab === "overview") {
                            void loadProjectOverview(slug);
                        }
                    }
                } else if (!selectionChangedDuringLoad) {
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
            setErrorStatus(e);
        }
    }

    /**
     * Re-derive the registry rows (status / missing_assets) for the project list.
     *
     * Knowledge generation writes into `.terrain/` outside of `refresh()`, so the
     * header list would otherwise keep serving the row computed at bootstrap and
     * keep showing 「待完善」 after the assets became ready.
     */
    async function refreshRegistryStatus() {
        try {
            project.registryProjects = await listRegistryProjects();
            const entry = findRegistryProject(
                project.selectedSlug,
                project.registryProjects,
            );
            if (entry) {
                project.selectedRepoPath = entry.repo_path;
            }
        } catch {
            /* keep the previous list when the registry cannot be re-read */
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
        project.freshnessLoadingSlug = requestSlug;
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
            if (project.freshnessLoadingSlug === requestSlug) {
                project.freshnessLoadingSlug = null;
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
        if (freshnessLoading) return;
        if (project.projectOverview?.freshness?.overall_score != null) return;
        void runFreshnessCompute(slug, repoPath);
    }

    async function loadProjectOverviewFreshness(
        slug: string,
        repoPath: string | null | undefined,
    ) {
        if (!repoPath) {
            if (project.freshnessLoadingSlug === slug) {
                project.freshnessLoadingSlug = null;
            }
            return;
        }
        freshnessComputeSlug = null;
        await applyCachedFreshness(slug);
        if (project.selectedSlug !== slug) return;
        scheduleFreshnessCompute(slug, repoPath);
    }

    async function loadProjectOverview(
        slug: string,
        opts?: { skipFreshness?: boolean },
    ) {
        const seq = nextLoadSeq(overviewLoadSeq, slug);
        const visible = project.selectedSlug === slug;
        if (visible) project.overviewLoading = true;
        try {
            const overview = await getProjectOverview(slug);
            // A slower reply for this project, or a reply for a project the user
            // already left, must not repaint the view.
            if (overviewLoadSeq.get(slug) !== seq) return;
            if (project.selectedSlug !== slug) return;
            project.projectOverview = overview;
            if (overview?.repo_path && !opts?.skipFreshness) {
                void loadProjectOverviewFreshness(slug, overview.repo_path);
            } else if (project.freshnessLoadingSlug === slug) {
                project.freshnessLoadingSlug = null;
            }
        } catch {
            if (overviewLoadSeq.get(slug) !== seq) return;
            if (project.selectedSlug !== slug) return;
            project.projectOverview = null;
            if (project.freshnessLoadingSlug === slug) {
                project.freshnessLoadingSlug = null;
            }
        } finally {
            if (
                visible &&
                overviewLoadSeq.get(slug) === seq &&
                project.selectedSlug === slug
            ) {
                project.overviewLoading = false;
            }
        }
    }

    async function loadHumanDocs(slug: string) {
        const seq = nextLoadSeq(humanDocsLoadSeq, slug);
        const visible = project.selectedSlug === slug;
        if (visible) project.humanDocsLoading = true;
        try {
            const docs = await listHumanDocs(slug);
            if (humanDocsLoadSeq.get(slug) !== seq) return;
            if (project.selectedSlug !== slug) return;
            project.humanDocs = docs;
        } catch {
            if (humanDocsLoadSeq.get(slug) !== seq) return;
            if (project.selectedSlug !== slug) return;
            project.humanDocs = [];
        } finally {
            if (
                visible &&
                humanDocsLoadSeq.get(slug) === seq &&
                project.selectedSlug === slug
            ) {
                project.humanDocsLoading = false;
            }
        }
    }

    /**
     * Opening the list is the moment the user reads the readiness badges, so re-derive
     * them first — knowledge may have been regenerated by the CLI or by a finished
     * background task since the last refresh.
     */
    function toggleProjectPicker() {
        project.pickerOpen = !project.pickerOpen;
        if (project.pickerOpen) {
            void refreshRegistryStatus();
        }
    }

    async function addProject() {
        project.pickerOpen = false;
        if (project.importJob) {
            setStatus(
                t("app.status.importBusy", { name: importProjectName }),
                "idle",
                null,
                STATUS_AUTO_DISMISS_MS,
            );
            return;
        }
        let picked: string | null;
        try {
            picked = await open({ directory: true, multiple: false });
        } catch (e) {
            setLocalizedErrorStatus(
                (summary) => t("app.status.pickFolderFailed", { error: summary }),
                e,
            );
            return;
        }
        if (!picked || Array.isArray(picked)) return;
        await triggerProjectInitialization(picked);
    }

    async function removeRegistryProject(entry: ProjectRegistryEntry) {
        project.pickerOpen = false;
        // A running import re-registers the project at the end, so removal in
        // that window would silently undo itself.
        if (
            project.importJob &&
            (project.importJob.slug === entry.slug ||
                project.importJob.repoPath === entry.repo_path)
        ) {
            setStatus(
                t("app.status.removeWhileImporting", {
                    name: registryDisplayName(entry),
                }),
                "idle",
                null,
                STATUS_AUTO_DISMISS_MS,
            );
            return;
        }
        try {
            await removeProject(entry.slug);
            clearProjectTask(entry.slug);
            if (project.selectedSlug === entry.slug) {
                project.selectedSlug = null;
                project.selectedRepoPath = null;
                project.projectOverview = null;
                project.activeDoc = null;
                project.activeHumanPath = null;
                project.humanDocs = [];
                project.hits = [];
                project.knowledgeRoot = "";
            }
            await refresh({ force: true });
            setStatus(
                t("app.status.removedFromList", {
                    name: registryDisplayName(entry),
                }),
                "success",
            );
        } catch (e) {
            setErrorStatus(e);
        }
    }

    function selectRegistryProject(entry: ProjectRegistryEntry) {
        project.pickerOpen = false;
        if (project.selectedSlug !== entry.slug) {
            project.activeDoc = null;
            project.activeHumanPath = null;
            project.hits = [];
            project.deepWikiInitialQuestion = null;
            project.overviewLoading = false;
            project.humanDocsLoading = false;
            project.docLoading = false;
        }
        project.selectedSlug = entry.slug;
        project.selectedRepoPath = entry.repo_path;
        project.activeTab = "overview";

        if (entry.status === "stale") {
            project.projectOverview = null;
            project.freshnessLoadingSlug = null;
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
            setLocalizedErrorStatus(
                (summary) => t("terms.msg.openFolderFailed", { error: summary }),
                e,
            );
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

    const showStatusBar = $derived(
        (status.kind !== "idle" || !isIdleStatusMessage(status.message)) &&
            // A bar already narrates the running task; keep errors/success visible.
            !(
                taskBar &&
                (status.kind === "progress" || status.kind === "loading")
            ),
    );

    /** Switch to a project by slug (the background-task bar offers this). */
    function focusProjectBySlug(slug: string) {
        const entry = findRegistryProject(slug, project.registryProjects);
        if (entry) selectRegistryProject(entry);
    }

    function openArchitectureDoc() {
        const slug = project.selectedSlug;
        const docPath = project.projectOverview?.agent_context.path;
        if (!docPath || !slug) return;
        void (async () => {
            project.activeTab = "knowledge";
            const seq = ++docLoadSeq;
            project.docLoading = true;
            try {
                await loadHumanDocs(slug);
                if (project.selectedSlug !== slug) return;
                const doc = await readDocument(docPath);
                if (project.selectedSlug !== slug) return;
                project.activeDoc = doc;
                project.activeHumanPath = docPath;
            } catch (e) {
                if (project.selectedSlug === slug) setErrorStatus(e);
            } finally {
                if (seq === docLoadSeq) project.docLoading = false;
            }
        })();
    }

    async function triggerProjectInitialization(
        repoPath: string,
        slug?: string,
    ) {
        // Imports are serialized: one scan + Litho + context run at a time.
        if (project.importJob) {
            setStatus(
                t("app.status.importBusy", { name: importProjectName }),
                "idle",
                null,
                STATUS_AUTO_DISMISS_MS,
            );
            return;
        }

        const focusSlug = slug ?? null;
        if (focusSlug && project.selectedSlug !== focusSlug) {
            // Re-initializing an existing project focuses it, so progress is
            // anchored to that project instead of the previously viewed one.
            const entry = findRegistryProject(
                focusSlug,
                project.registryProjects,
            );
            if (entry) {
                selectRegistryProject(entry);
            } else {
                project.selectedSlug = focusSlug;
                project.selectedRepoPath = repoPath;
                project.activeTab = "overview";
            }
        }

        const job = beginImport(repoPath, focusSlug);
        if (focusSlug) {
            setProjectTask(focusSlug, {
                repackBusy: true,
                lithoBusy: false,
                lithoProgress: "",
            });
        }

        try {
            const result = await initializeProject(repoPath, slug);
            updateImportJob(job.id, {
                slug: result.project_slug,
                stage: "done",
                message: t("misc.progress.stageDone"),
            });

            // Only steal focus when the user did not navigate away meanwhile.
            // Re-checked after `refresh` because that can also move the selection.
            const focusWanted =
                project.selectedSlug === job.selectionAtStart;
            await refresh({ force: true });

            const name = importDisplayName(
                result.project_slug,
                result.repo_path,
                project.registryProjects,
            );

            const viewingTarget =
                project.selectedSlug === result.project_slug;
            if (viewingTarget) {
                // Already looking at the imported project (re-init, or the
                // refresh auto-selected it). Reload its data in place instead of
                // re-selecting, so the current tab and view are preserved.
                void Promise.all([
                    loadHumanDocs(result.project_slug),
                    loadProjectOverview(result.project_slug),
                    refreshKnowledgeRoot(result.project_slug),
                ]);
            } else if (
                focusWanted &&
                project.selectedSlug === job.selectionAtStart
            ) {
                const entry = findRegistryProject(
                    result.project_slug,
                    project.registryProjects,
                );
                if (entry) {
                    selectRegistryProject(entry);
                } else {
                    project.selectedSlug = result.project_slug;
                    project.selectedRepoPath = result.repo_path;
                    await Promise.all([
                        loadHumanDocs(result.project_slug),
                        loadProjectOverview(result.project_slug),
                        refreshKnowledgeRoot(result.project_slug),
                    ]);
                }
            }

            const note = result.notes.length
                ? ` · ${result.notes.join("；")}`
                : "";
            const lithoNote =
                result.litho_ran && !result.human_docs_complete
                    ? ` · ${t("app.status.lithoIncomplete")}`
                    : "";
            setStatus(
                t("app.status.initComplete", {
                    name,
                    scan: result.scan_files_written,
                    term: t("terms.humanKnowledge"),
                    count: result.human_doc_count,
                    lithoNote,
                    note,
                }),
                result.notes.length || lithoNote ? "idle" : "success",
                result.project_slug,
            );
        } catch (e) {
            setErrorStatus(e);
        } finally {
            // Clear by the slug this run owns — never by the current selection,
            // which may belong to an unrelated project by now. `finishImport`
            // returns the live job, including the slug adopted from events.
            const cleared = finishImport(job.id);
            const clearedSlug = cleared?.slug ?? focusSlug;
            if (clearedSlug) {
                setProjectTask(clearedSlug, {
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
        const slug = project.selectedSlug;
        const repoPath = project.selectedRepoPath;
        if (project.projectTasks[slug]?.quickRefreshBusy) return;
        setProjectTask(slug, { quickRefreshBusy: true });
        setStatus(
            t("app.status.quickRefreshing"),
            "progress",
            slug,
        );
        try {
            const result = await runQuickRefresh(repoPath, slug);
            const note = result.notes.length
                ? ` · ${result.notes.join("；")}`
                : "";
            if (project.selectedSlug === slug) {
                setStatus(
                    t("app.status.refreshDone", {
                        score: result.freshness.overall_score,
                        note,
                    }),
                    result.freshness.overall_stale ? "idle" : "success",
                    slug,
                );
            }
            await loadProjectOverview(slug, { skipFreshness: true });
            if (project.selectedSlug === slug && project.projectOverview) {
                project.projectOverview = mergeFreshnessIntoOverview(
                    project.projectOverview,
                    result.freshness,
                );
                if (project.freshnessLoadingSlug === slug) {
                    project.freshnessLoadingSlug = null;
                }
            }
            await refreshRegistryStatus();
        } catch (e) {
            if (project.selectedSlug === slug) setErrorStatus(e);
        } finally {
            setProjectTask(slug, { quickRefreshBusy: false });
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
        const slug = project.selectedSlug;
        const repoPath = project.selectedRepoPath;
        if (project.projectTasks[slug]?.agentContextBusy) return;
        if (!project.acpOk && !hybridNativeLlm) {
            setStatus(t("app.configureAcp"), "error");
            return;
        }
        if (hybridNativeLlm && !project.llmStatus?.ready) {
            setStatus(t("app.configureLlm"), "error");
            return;
        }
        setProjectTask(slug, { agentContextBusy: true });
        setStatus(t("terms.msg.agentContextGenerating"), "progress", slug);
        try {
            // Overview 「生成」/「重新生成」 are explicit rebuild requests — never incremental.
            await runAgentContextGeneration(repoPath, slug, true);
            if (project.selectedSlug === slug) {
                setStatus(t("terms.msg.agentContextReady"), "success");
            }
            await Promise.all([
                loadProjectOverview(slug),
                loadHumanDocs(slug),
                refreshRegistryStatus(),
            ]);
        } catch (e) {
            if (project.selectedSlug === slug) setErrorStatus(e);
        } finally {
            setProjectTask(slug, { agentContextBusy: false });
        }
    }

    function openOverviewHumanDoc() {
        const slug = project.selectedSlug;
        if (!slug) return;
        const humanDir = project.projectOverview?.slug ?? slug;
        void (async () => {
            const docs = await listHumanDocs(humanDir);
            // Bail if the user moved on while the list was loading.
            if (project.selectedSlug !== slug) return;
            const overview = findHumanOverviewDoc(docs);
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
            const seq = ++docLoadSeq;
            project.docLoading = true;
            try {
                const docs = await listHumanDocs(slug);
                if (project.selectedSlug !== slug) return;
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
                if (project.selectedSlug === slug) setErrorStatus(e);
            } finally {
                if (seq === docLoadSeq) project.docLoading = false;
            }
        })();
    }

    async function openHumanDoc(doc: HumanDocEntry) {
        const slug = project.selectedSlug;
        project.activeHumanPath = doc.path;
        const seq = ++docLoadSeq;
        project.docLoading = true;
        setStatus(t("app.status.openingDoc", { title: doc.title }), "loading");
        try {
            const loaded = await readDocument(doc.path);
            if (project.selectedSlug !== slug) return;
            project.activeDoc = loaded;
            setStatus(t("app.status.viewingDoc", { title: doc.title }), "idle");
        } catch (e) {
            if (project.selectedSlug === slug) setErrorStatus(e);
        } finally {
            if (seq === docLoadSeq) project.docLoading = false;
        }
    }

    async function openDocPath(path: string) {
        const slug = project.selectedSlug;
        const seq = ++docLoadSeq;
        project.docLoading = true;
        try {
            const loaded = await readDocument(path);
            if (project.selectedSlug !== slug) return;
            project.activeDoc = loaded;
            project.activeHumanPath = path;
            project.deepWikiOpen = false;
        } catch (e) {
            if (project.selectedSlug === slug) setErrorStatus(e);
        } finally {
            if (seq === docLoadSeq) project.docLoading = false;
        }
    }

    async function packAgentForSelected() {
        const slug = project.selectedSlug;
        const repoPath = project.selectedRepoPath;
        if (!repoPath || !slug) return;
        if (project.projectTasks[slug]?.repackBusy) return;
        setProjectTask(slug, { repackBusy: true });
        setStatus(t("app.status.rebuildingIndex"), "progress", slug);
        try {
            const pack = await packAgentAssets(repoPath, slug);
            if (project.selectedSlug === slug) {
                setStatus(
                    t("app.status.indexUpdated", {
                        files: pack.total_files,
                        tokens: pack.total_tokens,
                    }),
                    "success",
                );
            }
            await loadProjectOverview(slug);
            await refreshRegistryStatus();
        } catch (e) {
            if (project.selectedSlug === slug) setErrorStatus(e);
        } finally {
            // Always release the project this run belongs to — even if the user
            // navigated away while it was running.
            setProjectTask(slug, { repackBusy: false });
        }
    }

    async function triggerHumanGeneration(forceRefresh?: boolean) {
        if (!project.selectedRepoPath || !project.selectedSlug) {
            setStatus(t("terms.msg.selectProjectWithRepoPath"), "error");
            return;
        }
        if (!project.acpOk && !hybridNativeLlm) {
            setStatus(t("app.status.acpNotFound"), "error");
            return;
        }
        if (hybridNativeLlm && !project.llmStatus?.ready) {
            setStatus(t("app.configureLlm"), "error");
            return;
        }
        const slug = project.selectedSlug;
        const repoPath = project.selectedRepoPath;
        if (project.projectTasks[slug]?.lithoBusy) return;
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
            await runLithoGeneration(repoPath, slug, force);
        } catch (e) {
            setErrorStatus(e);
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
            setErrorStatus(e);
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
            setErrorStatus(e);
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
        // In hybrid mode Ask runs on the native LLM, so a missing ACP binary does not block it.
        if (!project.acpOk && !hybridNativeLlm) {
            setStatus(t("app.configureAcp"), "error");
            return;
        }
        if (hybridNativeLlm && !project.llmStatus?.ready) {
            setStatus(t("app.configureLlm"), "error");
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
                setErrorStatus(e);
            }
        }
    }

    async function runSearch() {
        const q = project.query.trim();
        if (!q) {
            project.hits = [];
            return;
        }
        const slug = project.selectedSlug;
        const seq = ++docLoadSeq;
        project.docLoading = true;
        setStatus(t("app.status.searchingFor", { query: q }), "loading");
        try {
            const hits = await searchKnowledge(q, slug ?? undefined);
            if (project.selectedSlug !== slug) return;
            project.hits = hits;
            project.activeDoc = null;
            setStatus(
                `${hits.length} result(s)`,
                hits.length ? "success" : "idle",
            );
        } catch (e) {
            if (project.selectedSlug === slug) setErrorStatus(e);
        } finally {
            if (seq === docLoadSeq) project.docLoading = false;
        }
    }

    async function openHit(hit: SearchHit) {
        const slug = project.selectedSlug;
        const seq = ++docLoadSeq;
        project.docLoading = true;
        try {
            const loaded = await readDocument(hit.path);
            if (project.selectedSlug !== slug) return;
            project.activeDoc = loaded;
            project.hits = [];
        } catch (e) {
            if (project.selectedSlug === slug) setErrorStatus(e);
        } finally {
            if (seq === docLoadSeq) project.docLoading = false;
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
                const { project_slug, stage, message } = ev.payload;
                // Feeds the import job itself — that is what names the project in
                // the progress bar, independent of the current selection.
                applyImportProgress(project_slug, stage, message);
                if (stage === "human_docs") {
                    setProjectTask(project_slug, {
                        lithoBusy: true,
                        lithoProgress: message,
                    });
                } else if (stage === "scan") {
                    setProjectTask(project_slug, { repackBusy: true });
                } else if (stage === "done") {
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
                // Ignore progress for projects that were removed from the list.
                if (!isKnownProjectSlug(project_slug)) return;
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
                // Name the project, not its raw slug — this event may belong to a
                // project other than the selected one.
                const name = projectLabelForSlug(project_slug);
                const msg = !complete
                    ? count === 0
                        ? t("app.status.lithoDoneNoDocs", {
                              term: t("terms.humanKnowledge"),
                              name,
                          })
                        : t("app.status.lithoIncompleteWithCount", {
                              term: t("terms.humanKnowledge"),
                              name,
                              count,
                          })
                    : t("app.status.lithoReady", {
                          term: t("terms.humanKnowledge"),
                          name,
                          count,
                      });
                // The message names the project, so it stays accurate even when
                // the run belongs to a project the user is not viewing.
                setStatus(
                    msg,
                    complete ? "success" : count === 0 ? "error" : "idle",
                );
                if (project.selectedSlug === project_slug) {
                    await loadHumanDocs(project_slug);
                    await loadProjectOverview(project_slug);
                }
                await refreshRegistryStatus();
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
        class="flex w-16 shrink-0 flex-col items-stretch gap-2 border-r border-tr-border bg-tr-surface px-1.5 py-3"
    >
        <div
            class="mb-1 flex h-9 w-9 shrink-0 items-center justify-center self-center rounded-xl bg-gradient-to-br from-tr-accent to-tr-page text-tr-on-accent"
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
            class="flex flex-col items-center gap-1.5 self-center border-t border-tr-border pt-2"
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
                addBusy={importBusy}
                importingName={importProjectName || null}
                importingPath={importTargetRepoPath}
                importingSlug={importTargetSlug}
                ontoggle={toggleProjectPicker}
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

        {#if taskBar}
            <TaskProgressBar
                title={taskBar.title}
                projectName={taskBar.projectName}
                projectPath={taskBar.repoPath}
                stage={taskBar.stage}
                message={taskBar.message}
                steps={taskBar.steps}
                percent={taskBar.percent}
                note={taskBar.note}
                onswitch={taskBar.onswitch}
                switchLabel={taskBar.onswitch
                    ? tr("misc.progress.switchToProject")
                    : null}
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
            {agentContextBusy}
            {lithoBusy}
            {repackBusy}
            {importBusy}
            {importTargetSlug}
            {importTargetRepoPath}
            importProgress={importProgressMessage}
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
            {quickRefreshBusy}
            freshnessLoading={freshnessLoading}
            onQuickRefresh={triggerQuickRefresh}
            onRequestFreshnessCompute={requestFreshnessCompute}
            onSaveProjectRemark={async (remark) => {
                const slug = project.selectedSlug;
                if (!slug) return;
                const prevFreshness = project.projectOverview?.freshness;
                const updated = await saveProjectRemark(slug, remark);
                if (project.selectedSlug !== slug) return;
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
                        disabled={!project.selectedSlug || askUnavailable}
                        disabledReason={!project.selectedSlug
                            ? tr("app.selectProjectFirst")
                            : askUnavailable
                              ? (hybridNativeLlm
                                  ? tr("app.configureLlmTitle")
                                  : tr("app.configureAcpTitle"))
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
            // Hybrid mode only requires the LLM (ACP is optional — Litho falls back to it).
            const ok = hybridNativeLlm
                ? status.ready
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
