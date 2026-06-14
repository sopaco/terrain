export interface TocHeading {
  level: number;
  text: string;
  id: string;
}

const HEADING_LINE_RE = /^(#{1,6})\s+(.+?)\s*$/;

/** Strip common inline markdown for plain-text display and slug generation. */
export function stripMarkdownInline(text: string): string {
  return text
    .replace(/`([^`]+)`/g, "$1")
    .replace(/\*\*([^*]+)\*\*/g, "$1")
    .replace(/\*([^*]+)\*/g, "$1")
    .replace(/__([^_]+)__/g, "$1")
    .replace(/_([^_]+)_/g, "$1")
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .replace(/\\([\\`*_{}[\]()#+\-.!])/g, "$1")
    .trim();
}

export function slugifyHeading(text: string): string {
  const plain = stripMarkdownInline(text);
  const slug = plain
    .toLowerCase()
    .replace(/<[^>]+>/g, "")
    .replace(/[^\w\u4e00-\u9fff\s-]/g, "")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
  return slug || "section";
}

function createSlugAllocator(): (text: string) => string {
  const counts = new Map<string, number>();
  return (text: string) => {
    const base = slugifyHeading(text);
    const seen = counts.get(base) ?? 0;
    counts.set(base, seen + 1);
    return seen === 0 ? base : `${base}-${seen}`;
  };
}

/** Extract document headings (h1–h6), skipping fenced code blocks. */
export function extractMarkdownHeadings(markdown: string, maxLevel = 6): TocHeading[] {
  const allocateId = createSlugAllocator();
  const headings: TocHeading[] = [];
  let inFence = false;

  for (const line of markdown.split("\n")) {
    const trimmed = line.trimStart();
    if (trimmed.startsWith("```")) {
      inFence = !inFence;
      continue;
    }
    if (inFence) continue;

    const match = HEADING_LINE_RE.exec(line);
    if (!match) continue;

    const level = match[1].length;
    if (level > maxLevel) continue;

    const raw = match[2].trim();
    const text = stripMarkdownInline(raw);
    if (!text) continue;

    headings.push({
      level,
      text,
      id: allocateId(raw),
    });
  }

  return headings;
}

export interface MarkdownHeadingStructure {
  /** All heading ids in document order (for anchor injection). */
  headingIds: string[];
  /** Headings shown in the article TOC (defaults to h1–h4). */
  tocHeadings: TocHeading[];
}

/** Build anchor ids for the full document and a filtered TOC list for navigation. */
export function extractMarkdownHeadingStructure(
  markdown: string,
  tocMaxLevel = 4,
): MarkdownHeadingStructure {
  const all = extractMarkdownHeadings(markdown, 6);
  return {
    headingIds: all.map((h) => h.id),
    tocHeadings: all.filter((h) => h.level <= tocMaxLevel),
  };
}
