import type { CitationKind } from "./types";

/** Whether a path refers to a Litho / agent knowledge Markdown doc (not repomix source). */
export function isKnowledgeMarkdownPath(path: string): boolean {
  const p = path.trim().replace(/^\.\//, "").replace(/^\//, "");
  if (!p.endsWith(".md") || p.includes("repomix.md")) {
    return false;
  }
  if (p.includes("/.mind-mesh/")) {
    return true;
  }
  return (
    p === "context.md" ||
    p.endsWith("/context.md") ||
    p.startsWith("human/") ||
    p.includes("/human/") ||
    p.startsWith("agent/context") ||
    p.startsWith("modules/") ||
    p.startsWith("interfaces/") ||
    p.startsWith("routes/")
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

/** Resolve a citation path to a `read_document` argument (absolute or `.mind-mesh`-relative). */
export function resolveKnowledgeDocPath(_projectSlug: string, path: string): string {
  const p = path.trim().replace(/^\.\//, "");
  if (p.startsWith("/")) {
    return p;
  }
  const mindMeshIdx = p.indexOf("/.mind-mesh/");
  if (mindMeshIdx >= 0) {
    return p.slice(mindMeshIdx + "/.mind-mesh/".length);
  }
  if (p.startsWith(".mind-mesh/")) {
    return p.slice(".mind-mesh/".length);
  }
  if (p === "context.md") {
    return "agent/context.md";
  }
  return p;
}
