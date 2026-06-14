const THINK_OPEN = "<" + "think" + ">";
const THINK_CLOSE = "<" + "/" + "think" + ">";
const THINK_BLOCK_RE = new RegExp(
  THINK_OPEN.replace(/[.*+?^${}()|[\]\\]/g, "\\$&") +
    "[\\s\\S]*?" +
    THINK_CLOSE.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"),
  "gi",
);

/** Qwen-style reasoning block: `\\n...\\n`` at the start of the answer. */
const QWEN_REASONING_PREFIX_RE = /^`\n[\s\S]*?``(?!`)\s*/;

const MARKDOWN_FENCE_RE = /^```(?:markdown|md)\s*\n([\s\S]*?)\n```\s*$/i;

/** Remove model reasoning blocks that should not appear in the rendered answer. */
export function stripThinkingBlocks(text: string): string {
  let out = text.trim();
  for (;;) {
    const next = out.replace(THINK_BLOCK_RE, "");
    if (next === out) break;
    out = next;
  }
  return out.replace(QWEN_REASONING_PREFIX_RE, "").trim();
}

/** Unwrap a single outer ```markdown / ```md fenced block if present. */
export function unwrapMarkdownFence(text: string): string {
  const trimmed = text.trim();
  const match = trimmed.match(MARKDOWN_FENCE_RE);
  return match ? match[1].trim() : trimmed;
}

/** Drop preamble before the first markdown heading. */
export function extractMarkdownBody(text: string): string {
  const trimmed = text.trim();
  const idx = trimmed.indexOf("\n## ");
  if (idx >= 0) return trimmed.slice(idx + 1).trimStart();
  if (trimmed.startsWith("## ")) return trimmed;
  return trimmed;
}

/** Fix headings, lists, fences, and tables run together on one line. */
export function repairInlineSectionHeadings(text: string): string {
  let out = text;
  out = out.replace(/([。；;：:.!?])\s*(## )/g, "$1\n\n$2");
  out = out.replace(/([\u4e00-\u9fff\w`\)])\s*(## )/g, "$1\n\n$2");
  out = out.replace(/([^\n\r#])(#{2,6} )/g, "$1\n\n$2");
  out = out.replace(/([^\n\r])(#{3,6} )/g, "$1\n\n$2");
  out = out.replace(/(#{1,6} [^\n]+[\u4e00-\u9fff])([A-Za-z])/g, "$1\n\n$2");
  out = out.replace(/([^\n\r`])(```[\w]+)/g, "$1\n\n$2");
  out = out.replace(/([^\n\r`])(```)(?=[^\w\n\r])/g, "$1\n\n$2");
  out = out.replace(/```(#{1,6} )/g, "```\n\n$1");
  out = out.replace(/([^\n\d])(\d+\.\s+\*\*)/g, "$1\n\n$2");
  out = out.replace(/\|\|/g, "|\n|");
  out = out.replace(/(```[\w]+)([^\n\r\w])/g, "$1\n$2");
  out = out.replace(/(#{1,6} [^\n]+?\))([^\n#\s\d-])/g, "$1\n\n$2");
  out = out.replace(/([：:])(\| )/g, "$1\n\n$2");
  out = out.replace(/(#{1,6} [^\n]+?)(-\s*`)/g, "$1\n\n$2");
  out = out.replace(/([^\n\r])(-\s*`)/g, "$1\n\n$2");
  return out;
}

/** Re-insert markdown structure when providers stream a single flattened line. */
export function repairFlattenedMarkdown(text: string): string {
  const layout = repairInlineSectionHeadings(text);
  if ((layout.match(/\n/g) ?? []).length >= 3) return layout;

  let out = layout;
  out = out.replace(/\*\^([^^*]+)\^\*/g, "*$1*");
  out = out.replace(/\^([^^*]+)\^/g, "*$1*");
  out = out.replace(/(### [^*]+?)(\*\*[^*]+：[^*]*\*\*)/g, "$1\n\n$2");
  out = out.replace(/(\*\*[^*]+\*\*)([\u4e00-\u9fff])/g, "$1\n\n$2");
  out = out.replace(/([。！？])([\u4e00-\u9fff看当从不这无每])/g, "$1\n\n$2");
  out = out.replace(/(\*)([从当不无这每])/g, "$1\n\n$2");
  out = out.replace(/([。；;：:])(### )/g, "$1\n\n$2");
  out = out.replace(/([。`])(\*\*\d+\.\d+)/g, "$1\n\n$2");
  out = out.replace(/([。])(\*\*[^*]+\*\*：)/g, "$1\n\n$2");
  out = out.replace(/(\*\*)(\d+\.\s+\*\*)/g, "$1\n$2");
  out = out.replace(/([：:])(\d+\.\s+\*\*)/g, "$1\n$2");
  out = out.replace(/：```rustpub /g, "：\n\n```rust\npub ");
  out = out.replace(/```rustpub struct/g, "```rust\npub struct");
  out = out.replace(/\{ pub /g, "{\n pub ");
  out = out.replace(/, pub /g, ",\n pub ");
  out = out.replace(/\}```/g, "}\n```\n\n");
  out = out.replace(/(`\))([\u4e00-\u9fff])/g, "$1\n\n$2");
  out = out.replace(/(\))(\*\*[^*]+\*\*)([\u4e00-\u9fff])/g, "$1\n\n$2\n\n$3");
  out = out.replace(/(\))(\*\*[^*]+:\*\*)/g, "$1\n\n$2");
  out = out.replace(/(\*\*[^*]+:\*\*)(\d+\.)/g, "$1\n$2");
  return out;
}

/** Full cleanup before markdown rendering. */
export function prepareMarkdownForRender(text: string, opts?: { extractBody?: boolean }): string {
  const stripped = stripThinkingBlocks(text);
  const unwrapped = unwrapMarkdownFence(stripped);
  const repaired = repairFlattenedMarkdown(unwrapped);
  if (opts?.extractBody) return extractMarkdownBody(repaired);
  return repaired;
}

const FENCE_RE = /(```[\s\S]*?```)/g;
const INLINE_CODE_RE = /(`[^`\n]+`)/g;

/** Split markdown into fenced blocks, inline code spans, and plain text. */
function splitProtectedMarkdown(text: string): string[] {
  const parts: string[] = [];
  let last = 0;
  for (const match of text.matchAll(FENCE_RE)) {
    const idx = match.index ?? 0;
    if (idx > last) {
      parts.push(...splitInlineCode(text.slice(last, idx)));
    }
    parts.push(match[1]);
    last = idx + match[1].length;
  }
  if (last < text.length) {
    parts.push(...splitInlineCode(text.slice(last)));
  }
  return parts;
}

function splitInlineCode(text: string): string[] {
  const parts: string[] = [];
  let last = 0;
  for (const match of text.matchAll(INLINE_CODE_RE)) {
    const idx = match.index ?? 0;
    if (idx > last) {
      parts.push(text.slice(last, idx));
    }
    parts.push(match[1]);
    last = idx + match[1].length;
  }
  if (last < text.length) {
    parts.push(text.slice(last));
  }
  return parts;
}

/** Linkify file references only outside fenced and inline code spans. */
export function linkifySourcesOutsideCode(
  text: string,
  linkify: (segment: string) => string,
): string {
  return splitProtectedMarkdown(text)
    .map((part) => {
      if (part.startsWith("```") || (part.startsWith("`") && part.endsWith("`"))) {
        return part;
      }
      return linkify(part);
    })
    .join("");
}

const CODE_HTML_RE = /(<(?:pre|code)\b[^>]*>[\s\S]*?<\/(?:pre|code)>)/gi;

/** Linkify file references in rendered HTML, skipping code/pre elements. */
export function linkifySourcesInHtml(
  html: string,
  linkify: (segment: string) => string,
): string {
  return html
    .split(CODE_HTML_RE)
    .map((segment, index) => {
      if (index % 2 === 1) return segment;
      return segment
        .split(/(<[^>]+>)/g)
        .map((part) => (part.startsWith("<") ? part : linkify(part)))
        .join("");
    })
    .join("");
}
