import type { ProjectRegistryEntry, ProjectRegistryStatus } from "./types";

/** Last path segment of a repository directory (e.g. `money-never-sleep`). */
export function repoBasename(repoPath: string): string {
  const trimmed = repoPath.replace(/\/+$/, "");
  const idx = trimmed.lastIndexOf("/");
  return idx >= 0 ? trimmed.slice(idx + 1) : trimmed;
}

export function registryDisplayName(entry: ProjectRegistryEntry): string {
  return entry.name || repoBasename(entry.repo_path) || entry.slug;
}

export function findRegistryProject(
  slug: string | null | undefined,
  projects: ProjectRegistryEntry[],
): ProjectRegistryEntry | undefined {
  if (!slug) return undefined;
  return projects.find((p) => p.slug === slug);
}

export function selectedProjectDisplayName(
  slug: string | null,
  projects: ProjectRegistryEntry[],
  fallback: string,
): string {
  const entry = findRegistryProject(slug, projects);
  return entry ? registryDisplayName(entry) : fallback;
}

export function isRegistryStale(entry: ProjectRegistryEntry): boolean {
  return entry.status === "stale";
}

export function isRegistryPartial(entry: ProjectRegistryEntry): boolean {
  return entry.status === "partial";
}

export function isRegistryReady(entry: ProjectRegistryEntry): boolean {
  return entry.status === "ready";
}

export function statusBadgeLabel(status: ProjectRegistryStatus): string | null {
  switch (status) {
    case "stale":
      return "需修复";
    case "partial":
      return "待完善";
    default:
      return null;
  }
}

export function registryRepairDetail(entry: ProjectRegistryEntry): string {
  if (entry.status === "stale") {
    return `仓库 \`.terrain\` 已缺失或损坏（${entry.repo_path}），可一键重新扫描并生成知识资产。`;
  }
  const missing =
    entry.missing_assets.length > 0
      ? `尚未就绪：${entry.missing_assets.join("、")}。`
      : "部分知识资产尚未就绪。";
  return `${missing}（${entry.repo_path}）`;
}

/** Prefer ready → partial → stale when auto-selecting a project. */
export function preferredRegistryProject(
  projects: ProjectRegistryEntry[],
): ProjectRegistryEntry | undefined {
  return (
    projects.find((p) => p.status === "ready") ??
    projects.find((p) => p.status === "partial") ??
    projects[0]
  );
}

export function countRegistryByStatus(projects: ProjectRegistryEntry[]) {
  let ready = 0;
  let partial = 0;
  let stale = 0;
  for (const p of projects) {
    if (p.status === "ready") ready += 1;
    else if (p.status === "partial") partial += 1;
    else stale += 1;
  }
  return { ready, partial, stale };
}
