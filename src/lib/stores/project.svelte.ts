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
  graphifyBusy: boolean;
  graphifyProgress: string;
  uaBusy: boolean;
  uaProgress: string;
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
  projectOverview: null as ProjectOverview | null,
  overviewLoading: false,
  freshnessLoading: false,
  agentContextBusy: false,
  quickRefreshBusy: false,
  initBusy: false,
  initProgress: null as string | null,
  docLoading: false,
  activeTab: "overview" as AppTab,
  pickerOpen: false,
  settingsOpen: false,
  helpOpen: false,
  deepWikiOpen: false,
  deepWikiInitialQuestion: null as string | null,
});

export function setProjectTask(slug: string, patch: Partial<ProjectTaskState>) {
  const prev = project.projectTasks[slug] ?? {
    repackBusy: false,
    lithoBusy: false,
    lithoProgress: "",
    graphifyBusy: false,
    graphifyProgress: "",
    uaBusy: false,
    uaProgress: "",
  };
  project.projectTasks[slug] = { ...prev, ...patch };
}

export function currentTask(): ProjectTaskState {
  const empty: ProjectTaskState = {
    repackBusy: false,
    lithoBusy: false,
    lithoProgress: "",
    graphifyBusy: false,
    graphifyProgress: "",
    uaBusy: false,
    uaProgress: "",
  };
  if (!project.selectedSlug) return empty;
  return project.projectTasks[project.selectedSlug] ?? empty;
}

export function selectedRegistryProject() {
  return findRegistryProject(project.selectedSlug, project.registryProjects);
}

export function hasAnySelectableProject() {
  return project.registryProjects.length > 0;
}
