import type { CitationKind } from "./types";

/** Whether a path refers to a Litho / agent knowledge Markdown doc (not repomix source). */
export function isKnowledgeMarkdownPath(path: string): boolean {
  const p = path.trim().replace(/^\.\//, "").replace(/^\//, "");
  if (!p.endsWith(".md") || p.includes("repomix.md")) {
    return false;
  }
  return (
    p === "context.md" ||
    p.endsWith("/context.md") ||
    p.startsWith("human/") ||
    p.includes("/human/") ||
    p.startsWith("agent/context") ||
    p.startsWith("modules/") ||
    p.startsWith("interfaces/") ||
    p.startsWith("routes/") ||
    p.startsWith("projects/")
  );
}

export function citationKindForPath(path: string): CitationKind {
  if (!isKnowledgeMarkdownPath(path)) {
    return "source_code";
  }
  if (
    path.startsWith("modules/") ||
    path.startsWith("interfaces/") ||
    path.startsWith("routes/") ||
    path.includes("/modules/") ||
    path.includes("/interfaces/") ||
    path.includes("/routes/")
  ) {
    return "structured_doc";
  }
  return "human_doc";
}

/** Resolve a citation path to a `read_document` argument under the knowledge root. */
export function resolveKnowledgeDocPath(projectSlug: string, path: string): string {
  const p = path.trim().replace(/^\.\//, "").replace(/^\//, "");
  if (p.startsWith("projects/") || p.includes("/") && !p.startsWith("human/") && p !== "context.md") {
    if (p.startsWith(`projects/${projectSlug}/`)) {
      return p;
    }
  }
  if (p === "context.md") {
    return `projects/${projectSlug}/agent/context.md`;
  }
  if (p.startsWith("human/") || p.startsWith("agent/")) {
    return `projects/${projectSlug}/${p}`;
  }
  return p;
}
