export interface ProjectSummary {
  slug: string;
  name: string;
  path: string;
  repo_path?: string;
}

export interface StaleProjectSummary {
  slug: string;
  repo_path: string;
}

export interface ProjectInitResult {
  project_slug: string;
  repo_path: string;
  scan_files_written: number;
  repack_tokens?: number;
  agent_context_generated: boolean;
  human_doc_count: number;
  human_docs_complete: boolean;
  litho_ran: boolean;
  notes: string[];
}

export interface SearchHit {
  path: string;
  project: string;
  doc_type: string;
  title?: string;
  snippet: string;
  score: number;
}

export interface ScanReport {
  project_slug: string;
  files_written: number;
  collectors: string[];
  agent_pack?: {
    total_files: number;
    total_tokens: number;
    output_path: string;
  };
}

export interface KnowledgeDoc {
  path: string;
  frontmatter: Record<string, unknown>;
  body: string;
}

export interface AgentPackReport {
  project_slug: string;
  output_path: string;
  meta_path: string;
  total_files: number;
  total_tokens: number;
}

export interface LithoPlan {
  project_slug: string;
  repo_path: string;
  skill_dir: string;
  human_output_dir: string;
  litho_workspace_dir: string;
  skill_ready: boolean;
}

export interface LithoGenerationJob {
  plan: LithoPlan;
  prompt: string;
  acp_command: string;
  status: string;
}

export interface LithoGenerationResult {
  plan: LithoPlan;
  response_excerpt: string;
  human_doc_count: number;
  human_docs_complete: boolean;
}

export interface ProviderProfile {
  model?: string;
  api_key?: string;
  base_url?: string;
  ollama_host?: string;
}

export type AgentExecution = "native" | "acp";

/** @deprecated use AgentExecution */
export type AskExecution = AgentExecution;

export interface AcpSettings {
  binary?: string;
  args?: string;
  command?: string;
  agent_execution?: AgentExecution;
  auto_approve?: boolean;
}

export interface ModelSettings {
  provider?: string;
  model?: string;
  api_key?: string;
  base_url?: string;
  ollama_host?: string;
  profiles?: Record<string, ProviderProfile>;
  acp?: AcpSettings;
}

export interface LlmStatus {
  provider: string;
  model: string;
  ready: boolean;
  message: string;
  base_url?: string;
}

export interface HumanDocEntry {
  path: string;
  title: string;
  relative_path: string;
  /** Tree section: `human` or `agent`. */
  section?: string;
}

export type ChatPhase = "thinking" | "tools" | "generating" | "streaming";

export type CitationKind = "human_doc" | "structured_doc" | "source_code";

export interface SourceCitation {
  kind: CitationKind;
  title: string;
  path: string;
  repo_path?: string;
  start_line?: number;
  end_line?: number;
  excerpt?: string;
}

export interface SourceSlice {
  repo_path: string;
  file_path: string;
  start_line: number;
  end_line: number;
  content: string;
  format?: "code" | "markdown";
  /** Scroll to this line when opening a full-file view from a citation. */
  focus_line?: number;
}

export type ToolCallStatus = "running" | "ok" | "error";

export type AssistantStep =
  | { kind: "text"; content: string }
  | { kind: "tools"; toolCalls: ToolCallRecord[] };

export interface ToolCallRecord {
  id: string;
  name: string;
  arguments: Record<string, unknown>;
  result?: unknown;
  error?: string;
  status: ToolCallStatus;
  started_at: number;
  completed_at?: number;
  duration_ms?: number;
}

export interface TokenUsage {
  input_tokens: number;
  output_tokens: number;
  total_tokens: number;
  estimated?: boolean;
}

export interface AskKnowledgeReply {
  answer: string;
  citations: SourceCitation[];
  tool_calls: ToolCallRecord[];
  usage: TokenUsage;
  completed_at: number;
}

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
  citations?: SourceCitation[];
  toolCalls?: ToolCallRecord[];
  steps?: AssistantStep[];
  timestamp?: number;
  usage?: TokenUsage;
}

export type AppTab = "overview" | "knowledge" | "sdd" | "env";

export interface DocCounts {
  human: number;
  interfaces: number;
  routes: number;
  modules: number;
  events: number;
}

export interface LithoStatus {
  human_doc_count: number;
  has_human_docs: boolean;
  human_docs_complete: boolean;
  has_research_artifacts: boolean;
}

export interface AgentContextStatus {
  ready: boolean;
  path: string;
  excerpt?: string;
  generated_at?: string;
  section_count: number;
}

export interface AssetTrackHealth {
  track: string;
  label: string;
  ready: boolean;
  summary: string;
  detail: string;
  freshness_score?: number;
  stale?: boolean;
  stale_reason?: string;
}

export interface FreshnessDriftFactor {
  id: string;
  severity: "high" | "medium" | "low" | "info" | string;
  title: string;
  detail: string;
  points_lost?: number;
}

export interface FreshnessSummary {
  overall_score: number;
  overall_stale: boolean;
  commits_since_baseline: number;
  changed_files_count: number;
  current_git_head?: string;
  working_tree_dirty: boolean;
  is_git_repo: boolean;
  last_computed_at: string;
  stale_reason?: string;
  agent_pack_score: number;
  agent_context_score: number;
  human_docs_score: number;
  macro_preload_allowed: boolean;
  drift_factors?: FreshnessDriftFactor[];
  sample_changed_files?: string[];
  pack_baseline_short?: string;
  context_baseline_short?: string;
}

export interface QuickRefreshResult {
  project_slug: string;
  scan_files_written: number;
  pack_tokens?: number;
  agent_context_regenerated: boolean;
  notes: string[];
  freshness: FreshnessSummary;
}

export interface AgentEnvStatus {
  ready: boolean;
  integrated_count: number;
  total_count: number;
  summary: string;
  detail: string;
}

export interface AgentContextGenerationResult {
  output_path: string;
  meta: {
    project: string;
    repo_path: string;
    output_file: string;
    generated_at: string;
    section_count: number;
    char_count: number;
  };
  response_excerpt: string;
}

export interface AgentPackMeta {
  project: string;
  repo_path: string;
  generator: string;
  pack_strategy: string;
  output_file: string;
  total_files: number;
  total_tokens: number;
  total_characters: number;
  directory_structure: string;
  synced_at: string;
}

export interface ProjectOverview {
  slug: string;
  name: string;
  repo_path: string;
  tech_stack: string[];
  synced_at?: string;
  collectors: string[];
  doc_counts: DocCounts;
  agent_pack?: AgentPackMeta;
  litho: LithoStatus;
  agent_context: AgentContextStatus;
  asset_health: AssetTrackHealth[];
  agent_env: AgentEnvStatus;
  structure_preview?: string;
  overview_excerpt?: string;
  architecture_excerpt?: string;
  freshness?: FreshnessSummary;
}

export type SddPhase =
  | "requirements"
  | "tech_design"
  | "code_gen"
  | "code_review";

export interface SddPhaseInfo {
  phase: SddPhase;
  label: string;
  output_path: string;
  ready: boolean;
  updated_at?: string;
}

export interface SddSessionInfo {
  id: string;
  title: string;
  created_at: string;
  updated_at?: string;
}

export interface SddStatus {
  project_slug: string;
  skill_ready: boolean;
  workspace_dir: string;
  output_dir: string;
  phases: SddPhaseInfo[];
  current_phase?: SddPhase;
  active_session_id?: string;
  sessions: SddSessionInfo[];
}

export interface SddPhaseResult {
  phase: SddPhase;
  output_path: string;
  response_excerpt: string;
}

export interface EnvIntegrationStatus {
  id: string;
  kind: string;
  label: string;
  description: string;
  pub integrated: boolean;
  optional: boolean;
  /** MindMesh 安装包内置工具（RTK / CodeGraph 等） */
  bundled: boolean;
  /** 内置且可用时锁定勾选，不可取消 */
  locked: boolean;
  depends_on: string[];
  detail: string;
}

export interface EnvStatus {
  repo_path: string;
  ready_count: number;
  total_count: number;
  summary: string;
  items: EnvIntegrationStatus[];
}

export interface EnvPlanStep {
  id: string;
  label: string;
  kind: string;
  action: string;
}

export interface EnvPlan {
  repo_path: string;
  selected_ids: string[];
  steps: EnvPlanStep[];
  skipped: string[];
}

export interface EnvApplyResult {
  repo_path: string;
  applied: string[];
  skipped: string[];
  errors: string[];
  manifest_path: string;
}
