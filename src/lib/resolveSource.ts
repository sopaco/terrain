import { invoke } from "@tauri-apps/api/core";
import type { SourceCitation, SourceSlice } from "./types";
import {
  isKnowledgeMarkdownPath,
  resolveKnowledgeDocPath,
} from "./knowledgeDoc";

export interface ResolveSourceOptions {
  /** Load the entire file instead of a cited line range. */
  fullFile?: boolean;
}

export function isKnowledgeDocCitation(c: SourceCitation): boolean {
  return (
    c.kind === "human_doc" ||
    c.kind === "structured_doc" ||
    isKnowledgeMarkdownPath(c.path)
  );
}

export function knowledgeDocPathForCitation(
  projectSlug: string,
  path: string,
): string {
  if (path.startsWith("/") || path.includes(".mind-mesh/")) {
    return path;
  }
  return resolveKnowledgeDocPath(projectSlug, path);
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

/** Resolve a citation to a source panel slice (full file / document body). */
export async function citationToSourceSlice(
  projectSlug: string,
  citation: SourceCitation,
  repoPath: string | null | undefined,
  readDoc: (path: string) => Promise<{ body: string }>,
): Promise<SourceSlice> {
  if (isKnowledgeDocCitation(citation)) {
    const docPath = knowledgeDocPathForCitation(projectSlug, citation.path);
    const doc = await readDoc(docPath);
    return {
      repo_path: citation.repo_path ?? repoPath ?? "",
      file_path: citation.path,
      start_line: 0,
      end_line: 0,
      content: doc.body,
      format: "markdown",
    };
  }

  const slice = await resolveSourceCitation(projectSlug, citation, repoPath, {
    fullFile: true,
  });
  return { ...slice, format: slice.format ?? "code" };
}
