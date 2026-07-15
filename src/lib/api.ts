import { invoke } from "@tauri-apps/api/core";
import type {
  AgentContextGenerationResult,
  AgentPackReport,
  AskKnowledgeReply,
  EnvApplyResult,
  EnvPlan,
  EnvStatus,
  FreshnessSummary,
  HumanDocEntry,
  KnowledgeDoc,
  LithoGenerationJob,
  LithoPlan,
  LlmStatus,
  ModelSettings,
  ProjectInitResult,
  ProjectOverview,
  ProjectSummary,
  QuickRefreshResult,
  ScanReport,
  SddPhase,
  SddPhaseResult,
  SddSessionInfo,
  SddStatus,
  SearchHit,
  SourceSlice,
  StaleProjectSummary,
  UsageDetailLevel,
  UsageProbeResult,
  UsageSnapshot,
} from "./types";

export const listProjects = () => invoke<ProjectSummary[]>("list_projects");

export const listStaleProjects = () =>
  invoke<StaleProjectSummary[]>("list_stale_projects_cmd");

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
  requestId?: string,
) =>
  invoke<AskKnowledgeReply>("ask_knowledge_cmd", {
    query,
    project: project || null,
    repoPath: repoPath || null,
    requestId: requestId ?? crypto.randomUUID(),
  });

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

export const runAgentContextGeneration = (repoPath: string, projectSlug?: string) =>
  invoke<AgentContextGenerationResult>("run_agent_context_generation_cmd", {
    repoPath,
    projectSlug: projectSlug || null,
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

export const probeUsage = () => invoke<UsageProbeResult>("usage_probe_cmd");

export const getUsageSnapshot = (detail: UsageDetailLevel = "summary", forceRefresh = false) =>
  invoke<UsageSnapshot>("usage_snapshot_cmd", { detail, forceRefresh });
