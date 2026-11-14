import type {
  AgentExecution,
  AppTab,
  HumanDocEntry,
  KnowledgeDoc,
  LlmStatus,
  ProjectOverview,
  ProjectRegistryEntry,
  SearchHit,
} from "../types";
import { findRegistryProject } from "../projectRegistry";

export type ProjectTaskState = {
  repackBusy: boolean;
  lithoBusy: boolean;
  lithoProgress: string;
  agentContextBusy: boolean;
  quickRefreshBusy: boolean;
};

/**
 * One in-flight "add & initialize repository" run.
 *
 * Imports are serialized (see `beginImport`), but they are *not* tied to the
 * selected project: a freshly picked repository has no slug until the backend
 * scans it, and the user is free to browse other projects meanwhile. Keeping the
 * job in its own object is what lets the progress UI name the project actually
 * being imported instead of whatever happens to be selected.
 */
export type ImportJob = {
  /** Monotonic id — stale async replies must not clear a newer job. */
  id: number;
  repoPath: string;
  /** Resolved by the backend on the first progress event. */
  slug: string | null;
  /** `scan` | `human_docs` | `agent_context` | `done`. */
  stage: string | null;
  message: string | null;
  /** Stages actually reported by the backend, in order (skipped stages miss). */
  seenStages: string[];
  /** Selection when the run started — decides whether we may focus the result. */
  selectionAtStart: string | null;
};

const EMPTY_TASK: ProjectTaskState = {
  repackBusy: false,
  lithoBusy: false,
  lithoProgress: "",
  agentContextBusy: false,
  quickRefreshBusy: false,
};

export const project = $state({
  registryProjects: [] as ProjectRegistryEntry[],
  knowledgeRoot: "",
  acpOk: false,
  llmStatus: null as LlmStatus | null,
  agentExecution: "acp" as AgentExecution,
  query: "",
  hits: [] as SearchHit[],
  activeDoc: null as KnowledgeDoc | null,
  selectedSlug: null as string | null,
  selectedRepoPath: null as string | null,
  humanDocs: [] as HumanDocEntry[],
  activeHumanPath: null as string | null,
  humanDocsLoading: false,
  projectTasks: {} as Record<string, ProjectTaskState>,
  importJob: null as ImportJob | null,
  projectOverview: null as ProjectOverview | null,
  overviewLoading: false,
  /** Slug currently computing freshness — keep it per project, not global. */
  freshnessLoadingSlug: null as string | null,
  docLoading: false,
  activeTab: "overview" as AppTab,
  pickerOpen: false,
  settingsOpen: false,
  helpOpen: false,
  deepWikiOpen: false,
  deepWikiInitialQuestion: null as string | null,
});

export function setProjectTask(slug: string, patch: Partial<ProjectTaskState>) {
  const prev = project.projectTasks[slug] ?? EMPTY_TASK;
  project.projectTasks[slug] = { ...prev, ...patch };
}

export function clearProjectTask(slug: string) {
  delete project.projectTasks[slug];
}

export function taskFor(slug: string | null | undefined): ProjectTaskState {
  if (!slug) return EMPTY_TASK;
  return project.projectTasks[slug] ?? EMPTY_TASK;
}

export function currentTask(): ProjectTaskState {
  return taskFor(project.selectedSlug);
}

/**
 * A Litho run that is still emitting progress for a project other than the
 * selected one. Switching projects must not hide it (and must never relabel it
 * with the newly selected project's name).
 */
export function backgroundLithoTask(): { slug: string; progress: string } | null {
  for (const [slug, task] of Object.entries(project.projectTasks)) {
    if (slug !== project.selectedSlug && task.lithoProgress) {
      return { slug, progress: task.lithoProgress };
    }
  }
  return null;
}

let importSequence = 0;

/** Start an import job. Callers must guard on `project.importJob` first. */
export function beginImport(repoPath: string, slug: string | null): ImportJob {
  const job: ImportJob = {
    id: ++importSequence,
    repoPath,
    slug,
    stage: null,
    message: null,
    seenStages: [],
    selectionAtStart: project.selectedSlug,
  };
  project.importJob = job;
  return job;
}

export function updateImportJob(
  id: number,
  patch: Partial<Pick<ImportJob, "slug" | "stage" | "message">>,
) {
  const job = project.importJob;
  if (!job || job.id !== id) return;
  project.importJob = { ...job, ...patch };
}

/**
 * Apply a backend progress event. The first event carries the slug the backend
 * resolved, so a fresh import learns its identity before it finishes.
 */
export function applyImportProgress(
  slug: string,
  stage: string,
  message: string,
) {
  const job = project.importJob;
  if (!job) return;
  // Events from a superseded run must not hijack the current job.
  if (job.slug && job.slug !== slug) return;
  const seenStages =
    stage === "done" || job.seenStages.includes(stage)
      ? job.seenStages
      : [...job.seenStages, stage];
  project.importJob = {
    ...job,
    slug: job.slug ?? slug,
    stage,
    message,
    seenStages,
  };
}

/** Clear the job only when `id` still identifies it. Returns the cleared job. */
export function finishImport(id: number): ImportJob | null {
  const job = project.importJob;
  if (!job || job.id !== id) return null;
  project.importJob = null;
  return job;
}

/** Registry rows plus the project currently being imported. */
export function isKnownProjectSlug(slug: string): boolean {
  return (
    Boolean(findRegistryProject(slug, project.registryProjects)) ||
    project.importJob?.slug === slug
  );
}

export function selectedRegistryProject() {
  return findRegistryProject(project.selectedSlug, project.registryProjects);
}

export function hasAnySelectableProject() {
  return project.registryProjects.length > 0;
}
