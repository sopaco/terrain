import type { SourceCitation } from "./types";
import { citationKindForPath } from "./knowledgeDoc";

/** File extensions treated as linkable source / doc paths in Markdown. */
export const SOURCE_FILE_EXTENSIONS =
  "rs|ts|tsx|js|jsx|mjs|cjs|py|go|java|kt|swift|cs|cpp|c|h|md|yaml|yml|toml|json|svelte";

const EXT_GROUP = `(?:${SOURCE_FILE_EXTENSIONS})`;

/** Path segment in prose (optional surrounding backticks). */
export const SOURCE_PATH_SEGMENT_RE = new RegExp(
  "`?([a-zA-Z0-9_./-]+\\." +
    EXT_GROUP +
    ")(?::(\\d+)(?:[-–](\\d+))?)?`?",
  "g",
);

/** Entire inline-code span must be a single path reference. */
const SOURCE_INLINE_CODE_RE = new RegExp(
  "^`?([a-zA-Z0-9_./-]+\\." + EXT_GROUP + ")(?::(\\d+)(?:[-–](\\d+))?)?`?$",
);

export interface ParsedSourceRef {
  path: string;
  start?: number;
  end?: number;
}

export function parseSourceRef(text: string): ParsedSourceRef | null {
  const trimmed = text.trim();
  const match = trimmed.match(SOURCE_INLINE_CODE_RE);
  if (!match) return null;
  const path = match[1];
  const start = match[2] ? Number(match[2]) : undefined;
  const end = match[3] ? Number(match[3]) : start;
  return { path, start, end };
}

export function buildSourceCitation(
  parsed: ParsedSourceRef,
  repoPath?: string | null,
): SourceCitation {
  const { path, start, end } = parsed;
  return {
    kind: citationKindForPath(path),
    title: start ? `${path}:${start}` : path,
    path,
    repo_path: repoPath ?? undefined,
    start_line: start,
    end_line: end,
  };
}

export function linkifySourceSegments(
  text: string,
  linkify: (match: string, parsed: ParsedSourceRef) => string,
): string {
  return text.replace(SOURCE_PATH_SEGMENT_RE, (match, path: string, start?: string, end?: string) => {
    const parsed: ParsedSourceRef = {
      path,
      start: start ? Number(start) : undefined,
      end: end ? Number(end) : start ? Number(start) : undefined,
    };
    return linkify(match, parsed);
  });
}

export function sourceRefDataAttr(parsed: ParsedSourceRef): string {
  return encodeURIComponent(JSON.stringify(parsed));
}
