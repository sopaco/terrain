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
  };
  project.projectTasks[slug] = { ...prev, ...patch };
}

export function currentTask(): ProjectTaskState {
  if (!project.selectedSlug) {
    return { repackBusy: false, lithoBusy: false, lithoProgress: "" };
  }
  return (
    project.projectTasks[project.selectedSlug] ?? {
      repackBusy: false,
      lithoBusy: false,
      lithoProgress: "",
    }
  );
}

export function selectedRegistryProject() {
  return findRegistryProject(project.selectedSlug, project.registryProjects);
}

export function hasAnySelectableProject() {
  return project.registryProjects.length > 0;
}
