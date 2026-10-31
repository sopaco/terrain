import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  AgentContextGenerationResult,
  AgentPackReport,
  AppBootstrap,
  AskKnowledgeReply,
  AskSessionInfo,
  EnvApplyResult,
  EnvPlan,
  EnvStatus,
  AddProjectsResult,
  BatchUpdateResult,
  DashboardSummary,
  FreshnessSummary,
  GeneratedCommitMessage,
  GitBranches,
  GitCommit,
  GitDiff,
  GitFileEntry,
  GitMergePreview,
  GitOpResult,
  GitStatus,
  GraphifyCommunity,
  GraphifyStatus,
  GraphifyViewPayload,
  HumanDocEntry,
  KnowledgeDoc,
  LithoGenerationJob,
  LithoPlan,
  LlmStatus,
  UaStatus,
  UaViewerInfo,
  ModelSettings,
  ProjectInitResult,
  ProjectOverview,
  ProjectRegistryEntry,
  QuickRefreshResult,
  ScanReport,
  SddPhase,
  SddPhaseResult,
  SddSessionInfo,
  SddStatus,
  SearchHit,
  SourceSlice,
  UsageDetailLevel,
  UsageProbeResult,
  UsageSnapshot,
} from "./types";
import type { AskStreamEvent, ChatMessage } from "./types";

export const bootstrapApp = () => invoke<AppBootstrap>("bootstrap_app");

export const listRegistryProjects = () =>
  invoke<ProjectRegistryEntry[]>("list_registry_projects_cmd");

export const initializeProject = (repoPath: string, projectSlug?: string) =>
  invoke<ProjectInitResult>("initialize_project_cmd", {
    repoPath,
    projectSlug: projectSlug || null,
  });

export const scanProject = (repoPath: string, projectSlug?: string) =>
  invoke<ScanReport>("scan_project", {
    repoPath,
    projectSlug: projectSlug || null,
  });

export const packAgentAssets = (repoPath: string, projectSlug?: string) =>
  invoke<AgentPackReport>("pack_agent_assets_cmd", {
    repoPath,
    projectSlug: projectSlug || null,
  });

export const planLitho = (repoPath: string, projectSlug?: string) =>
  invoke<LithoPlan>("plan_litho_cmd", {
    repoPath,
    projectSlug: projectSlug || null,
  });

export const generateHumanDocs = (repoPath: string, projectSlug?: string) =>
  invoke<LithoGenerationJob>("generate_human_docs_cmd", {
    repoPath,
    projectSlug: projectSlug || null,
  });

export const runLithoGeneration = (
  repoPath: string,
  projectSlug?: string,
  forceRefresh = false,
) =>
  invoke<void>("run_litho_generation_cmd", {
    repoPath,
    projectSlug: projectSlug || null,
    forceRefresh,
  });

export const checkLlm = () => invoke<LlmStatus>("check_llm");

/** Real network probe: one minimal request against the configured endpoint. */
export const testLlm = () => invoke<LlmStatus>("test_llm_cmd");

/** Model ids served by the provider endpoint (sorted alphabetically). */
export const listProviderModels = (
  provider: string,
  baseUrl: string | null,
  apiKey: string | null,
  ollamaHost: string | null,
) =>
  invoke<string[]>("list_provider_models_cmd", {
    provider,
    baseUrl,
    apiKey,
    ollamaHost,
  });

export const getModelSettings = () => invoke<ModelSettings>("get_model_settings");

export const saveModelSettings = (settings: ModelSettings) =>
  invoke<LlmStatus>("save_model_settings_cmd", { settings });

export const listHumanDocs = (projectSlug: string) =>
  invoke<HumanDocEntry[]>("list_human_docs_cmd", { projectSlug });

export const openRepoFolder = (path: string) =>
  invoke<void>("open_repo_folder_cmd", { path });

export const readSourceSlice = (
  repoPath: string,
  filePath: string,
  startLine: number,
  endLine: number,
) =>
  invoke<SourceSlice>("read_source_slice_cmd", {
    repoPath,
    filePath,
    startLine,
    endLine,
  });

export const askKnowledge = (
  query: string,
  project?: string,
  repoPath?: string,
  onStream?: (event: AskStreamEvent) => void,
) => {
  const streamChannel = new Channel<AskStreamEvent>();
  if (onStream) {
    streamChannel.onmessage = onStream;
  }
  return invoke<AskKnowledgeReply>("ask_knowledge_cmd", {
    query,
    project: project || null,
    repoPath: repoPath || null,
    onStream: streamChannel,
  });
};

export const listAskSessions = (projectSlug: string) =>
  invoke<AskSessionInfo[]>("list_ask_sessions_cmd", { projectSlug });

export const loadAskMessages = (projectSlug: string, sessionId: string) =>
  invoke<ChatMessage[]>("load_ask_messages_cmd", { projectSlug, sessionId });

export const saveAskMessages = (
  projectSlug: string,
  sessionId: string,
  messages: ChatMessage[],
  firstQuestion?: string | null,
) =>
  invoke<AskSessionInfo>("save_ask_messages_cmd", {
    projectSlug,
    sessionId,
    messages,
    firstQuestion: firstQuestion ?? null,
  });

export const createAskSession = (projectSlug: string, question: string) =>
  invoke<AskSessionInfo>("create_ask_session_cmd", { projectSlug, question });

export const setActiveAskSession = (projectSlug: string, sessionId: string) =>
  invoke<AskSessionInfo[]>("set_active_ask_session_cmd", { projectSlug, sessionId });

export const deleteAskSession = (projectSlug: string, sessionId: string) =>
  invoke<AskSessionInfo[]>("delete_ask_session_cmd", { projectSlug, sessionId });

export const discardAskSession = (projectSlug: string, sessionId: string) =>
  invoke<AskSessionInfo[]>("discard_ask_session_cmd", { projectSlug, sessionId });

export const clearActiveAskSession = (projectSlug: string) =>
  invoke<void>("clear_active_ask_session_cmd", { projectSlug });

export const getActiveAskSession = (projectSlug: string) =>
  invoke<string | null>("get_active_ask_session_cmd", { projectSlug });

export const searchKnowledge = (query: string, project?: string) =>
  invoke<SearchHit[]>("search_knowledge", {
    query,
    project: project || null,
    limit: 20,
  });

export const readDocument = (path: string) =>
  invoke<KnowledgeDoc>("read_document", { path });

export const getKnowledgeRoot = (projectSlug?: string) =>
  invoke<string>("get_knowledge_root", {
    projectSlug: projectSlug || null,
  });

export const checkAcp = () => invoke<boolean>("check_acp");

/** @deprecated use checkAcp */
export const checkOpencode = () => invoke<boolean>("check_opencode");

export const getAcpSpawnCommand = () => invoke<string>("acp_spawn_command_cmd");

export const getProjectOverview = (projectSlug: string) =>
  invoke<ProjectOverview>("get_project_overview_cmd", { projectSlug });

export const saveProjectRemark = (projectSlug: string, remark: string) =>
  invoke<ProjectOverview>("save_project_remark_cmd", { projectSlug, remark });

export const computeFreshness = (projectSlug: string, repoPath?: string) =>
  invoke<FreshnessSummary>("compute_freshness_cmd", {
    projectSlug,
    repoPath: repoPath || null,
  });

export const readProjectFreshnessCached = (projectSlug: string) =>
  invoke<FreshnessSummary | null>("read_project_freshness_cached_cmd", { projectSlug });

export const runQuickRefresh = (repoPath: string, projectSlug?: string) =>
  invoke<QuickRefreshResult>("run_quick_refresh_cmd", {
    repoPath,
    projectSlug: projectSlug || null,
  });

export const getSddStatus = (projectSlug: string) =>
  invoke<SddStatus>("get_sdd_status_cmd", { projectSlug });

export const createSddSession = (projectSlug: string, title: string) =>
  invoke<SddSessionInfo>("create_sdd_session_cmd", { projectSlug, title });

export const setActiveSddSession = (projectSlug: string, sessionId: string) =>
  invoke<SddStatus>("set_active_sdd_session_cmd", { projectSlug, sessionId });

export const deleteSddSession = (projectSlug: string, sessionId: string) =>
  invoke<SddStatus>("delete_sdd_session_cmd", { projectSlug, sessionId });

export const saveSddOutput = (outputPath: string, content: string) =>
  invoke<void>("save_sdd_output_cmd", { outputPath, content });

export const removeProject = (projectSlug: string) =>
  invoke<void>("remove_project_cmd", { projectSlug });

/** `forceFull` bypasses incremental refresh — what the 「重新生成」 buttons request. */
export const runAgentContextGeneration = (
  repoPath: string,
  projectSlug?: string,
  forceFull = false,
) =>
  invoke<AgentContextGenerationResult>("run_agent_context_generation_cmd", {
    repoPath,
    projectSlug: projectSlug || null,
    forceFull,
  });

export const runSddPhase = (
  repoPath: string,
  phase: SddPhase,
  projectSlug?: string,
  userInput?: string,
  sessionId?: string,
) =>
  invoke<SddPhaseResult>("run_sdd_phase_cmd", {
    repoPath,
    projectSlug: projectSlug || null,
    sessionId: sessionId || null,
    phase,
    userInput: userInput || null,
  });

export const getEnvStatus = (repoPath: string) =>
  invoke<EnvStatus>("get_env_status_cmd", { repoPath });

export const planEnvIntegration = (
  repoPath: string,
  selectedIds: string[],
  reinstallIds: string[] = [],
) =>
  invoke<EnvPlan>("plan_env_integration_cmd", { repoPath, selectedIds, reinstallIds });

export const runEnvIntegration = (
  repoPath: string,
  selectedIds: string[],
  reinstallIds: string[] = [],
) =>
  invoke<EnvApplyResult>("run_env_integration_cmd", { repoPath, selectedIds, reinstallIds });

export const getDashboardSummary = () =>
  invoke<DashboardSummary>("get_dashboard_summary_cmd");

export const runDashboardBatch = (
  projectSlugs: string[] | null,
  concurrency?: number,
) =>
  invoke<BatchUpdateResult>("run_dashboard_batch_cmd", {
    projectSlugs,
    concurrency: concurrency ?? null,
  });

export const addProjects = (repoPaths: string[]) =>
  invoke<AddProjectsResult>("add_projects_cmd", { repoPaths });

export const getGraphifyStatus = (repoPath: string) =>
  invoke<GraphifyStatus>("graphify_status_cmd", { repoPath });

export const runGraphify = (
  repoPath: string,
  projectSlug: string | null,
  action: "extract" | "update",
) =>
  invoke<void>("run_graphify_cmd", { repoPath, projectSlug, action });

export const getGraphifyView = (repoPath: string) =>
  invoke<GraphifyViewPayload>("get_graphify_view_cmd", { repoPath });

export const getUaStatus = (repoPath: string) =>
  invoke<UaStatus>("ua_status_cmd", { repoPath });

export const startUaViewer = (repoPath: string) =>
  invoke<UaViewerInfo>("ua_viewer_cmd", { repoPath });

export const probeUsage = () => invoke<UsageProbeResult>("usage_probe_cmd");

export const getUsageSnapshot = (detail: UsageDetailLevel = "summary", forceRefresh = false) =>
  invoke<UsageSnapshot>("usage_snapshot_cmd", { detail, forceRefresh });

export const openLocalPath = (path: string) =>
  invoke<void>("open_local_path_cmd", { path });

export const getGitStatus = (repoPath: string) =>
  invoke<GitStatus>("git_status_cmd", { repoPath });

export const getGitBranches = (repoPath: string) =>
  invoke<GitBranches>("git_branches_cmd", { repoPath });

export const gitStage = (
  repoPath: string,
  paths: string[] | null,
  all: boolean,
) => invoke<GitOpResult>("git_stage_cmd", { repoPath, paths, all });

export const gitUnstage = (
  repoPath: string,
  paths: string[] | null,
  all: boolean,
) => invoke<GitOpResult>("git_unstage_cmd", { repoPath, paths, all });

export const gitCommit = (repoPath: string, message: string) =>
  invoke<GitOpResult>("git_commit_cmd", { repoPath, message });

export const gitMerge = (repoPath: string, branch: string) =>
  invoke<GitOpResult>("git_merge_cmd", { repoPath, branch });

export const getGitMergePreview = (repoPath: string, branch: string) =>
  invoke<GitMergePreview>("git_merge_preview_cmd", { repoPath, branch });

export const gitPush = (repoPath: string) =>
  invoke<GitOpResult>("git_push_cmd", { repoPath });

export const gitPull = (repoPath: string) =>
  invoke<GitOpResult>("git_pull_cmd", { repoPath });

export const getGitDiffFile = (repoPath: string, path: string, staged?: boolean) =>
  invoke<GitDiff>("git_diff_file_cmd", { repoPath, path, staged: staged ?? false });

export const getGitHistory = (repoPath: string, limit?: number) =>
  invoke<GitCommit[]>("git_history_cmd", { repoPath, limit: limit ?? null });

export const getGitCommitFilesList = (repoPath: string, hash: string) =>
  invoke<GitFileEntry[]>("git_commit_files_list_cmd", { repoPath, hash });

export const getGitCommitDiff = (repoPath: string, hash: string, path: string | null) =>
  invoke<GitDiff>("git_commit_diff_cmd", { repoPath, hash, path });

export const gitCommitFiles = (
  repoPath: string,
  summary: string,
  description: string | null,
  files: string[],
) =>
  invoke<GitOpResult>("git_commit_files_cmd", {
    repoPath,
    summary,
    description,
    files,
  });

export const gitFetch = (repoPath: string) =>
  invoke<GitOpResult>("git_fetch_cmd", { repoPath });

export const gitCheckout = (repoPath: string, branch: string) =>
  invoke<GitOpResult>("git_checkout_cmd", { repoPath, branch });

export const gitGenerateCommitMessage = (repoPath: string, files: string[]) =>
  invoke<GeneratedCommitMessage>("git_generate_commit_message_cmd", { repoPath, files });

export const gitDiscardFile = (repoPath: string, path: string) =>
  invoke<GitOpResult>("git_discard_file_cmd", { repoPath, path });

export const gitUndoLastCommit = (repoPath: string) =>
  invoke<GitOpResult>("git_undo_last_commit_cmd", { repoPath });

export const gitStashPush = (repoPath: string) =>
  invoke<GitOpResult>("git_stash_push_cmd", { repoPath });

export const gitStashPop = (repoPath: string) =>
  invoke<GitOpResult>("git_stash_pop_cmd", { repoPath });

export const gitCreateBranch = (repoPath: string, name: string) =>
  invoke<GitOpResult>("git_create_branch_cmd", { repoPath, name });
