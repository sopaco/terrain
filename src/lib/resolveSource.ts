import { invoke } from "@tauri-apps/api/core";
import type { SourceCitation, SourceSlice } from "./types";
import { isKnowledgeMarkdownPath } from "./knowledgeDoc";

export interface ResolveSourceOptions {
  /** Load the entire file instead of a cited line range. */
  fullFile?: boolean;
}

export async function resolveSourceCitation(
  projectSlug: string,
  citation: SourceCitation,
  repoPath?: string | null,
  options?: ResolveSourceOptions,
): Promise<SourceSlice> {
  const fullFile = options?.fullFile ?? false;
  const slice = await invoke<SourceSlice>("resolve_source_citation_cmd", {
    projectSlug,
    filePath: citation.path,
    startLine: fullFile ? null : (citation.start_line ?? null),
    endLine: fullFile ? null : (citation.end_line ?? null),
    repoPath: citation.repo_path ?? repoPath ?? null,
  });

  const isPackIndex = citation.path === "agent/repomix.md" || citation.path.endsWith("/agent/repomix.md");
  const isMarkdownDoc =
    isKnowledgeMarkdownPath(citation.path) ||
    (citation.path.endsWith(".md") && citation.kind !== "source_code" && !isPackIndex);

  const focusLine = fullFile ? citation.start_line : undefined;

  return {
    ...slice,
    format: isPackIndex || isMarkdownDoc ? "markdown" : "code",
    focus_line: focusLine,
  };
}
