import { describe, expect, test } from "bun:test";
import { marked } from "marked";
import {
  extractMarkdownBody,
  linkifySourcesInHtml,
  linkifySourcesOutsideCode,
  prepareMarkdownForRender,
  repairFlattenedMarkdown,
  stripThinkingBlocks,
  unwrapMarkdownFence,
} from "./markdownSanitize";

const SOURCE_RE =
  /`?([a-zA-Z0-9_./-]+\.(?:rs|ts|tsx|js|jsx|py|go|java|kt|swift|cs|cpp|c|h|md|yaml|yml|toml|json))(?::(\d+)(?:-(\d+))?)?`?/g;

function linkify(text: string): string {
  return text.replace(SOURCE_RE, (match) => `<button>${match}</button>`);
}

describe("stripThinkingBlocks", () => {
  test("removes think tags", () => {
    const open = "<" + "think" + ">";
    const close = "<" + "/" + "think" + ">";
    const input = `Hello${open}secret${close}World`;
    expect(stripThinkingBlocks(input)).toBe("HelloWorld");
  });

  test("removes qwen reasoning prefix only", () => {
    const input = "`\nLet me reason here\n``\n\n## Answer\n\nDone.";
    expect(stripThinkingBlocks(input)).toBe("## Answer\n\nDone.");
  });

  test("does not strip markdown code fences", () => {
    const input = "```markdown\n## Answer\n\nDone.\n```";
    expect(stripThinkingBlocks(input)).toBe(input);
  });

  test("does not strip inline code followed by fenced code", () => {
    const input = [
      "## Answer",
      "",
      "Use `read_file` helper.",
      "",
      "```rust",
      "fn main() {}",
      "```",
      "",
      "More `inline` code.",
    ].join("\n");
    expect(stripThinkingBlocks(input)).toBe(input);
  });
});

describe("unwrapMarkdownFence", () => {
  test("unwraps markdown code fence", () => {
    const input = "```markdown\n## Title\n\nBody\n```";
    expect(unwrapMarkdownFence(input)).toBe("## Title\n\nBody");
  });
});

describe("extractMarkdownBody", () => {
  test("drops preamble before first heading", () => {
    const input = "Here is the document.\n\n## Summary\n\nHello";
    expect(extractMarkdownBody(input)).toBe("## Summary\n\nHello");
  });
});

describe("linkifySourcesOutsideCode", () => {
  test("skips inline and fenced code", () => {
    const input = "See `src/foo.rs:1` and `src/bar.rs:2`.\n\n```\nsrc/baz.rs:3\n```";
    const linked = linkifySourcesOutsideCode(input, linkify);
    expect(linked).toContain("`src/foo.rs:1`");
    expect(linked).toContain("`src/bar.rs:2`");
    expect(linked).toContain("src/baz.rs:3");
    expect(linked).not.toContain("<button>");
  });
});

describe("markdown pipeline", () => {
  test("renders lists, code fences, and source refs", () => {
    const input = [
      "## Summary",
      "",
      "- Point A",
      "",
      "See src/lib/foo.rs:10 for details.",
      "",
      "```rust",
      "fn main() {}",
      "```",
    ].join("\n");

    const html = linkifySourcesInHtml(
      marked.parse(prepareMarkdownForRender(input), { async: false }) as string,
      linkify,
    );

    expect(html).toContain("<h2>Summary</h2>");
    expect(html).toContain("<li>Point A</li>");
    expect(html).toContain("<pre><code");
    expect(html).toContain("fn main()");
    expect(html).toContain("<button>src/lib/foo.rs:10</button>");
  });

  test("renders fenced markdown replies as markdown", () => {
    const input = "```markdown\n## Answer\n\n- Item one\n```";
    const html = marked.parse(prepareMarkdownForRender(input), { async: false }) as string;
    expect(html).toContain("<h2>Answer</h2>");
    expect(html).toContain("<li>Item one</li>");
    expect(html).not.toContain("<pre><code");
  });

  test("repairs flattened llm provider ask markdown", () => {
    const input =
      "说明如下：## 核心依赖与协议MindMesh **不直接实现** 客户端：1. **One**2. **Two**特性```toml\nkey = 1\n```## 总结Done。";
    const out = prepareMarkdownForRender(input);
    expect(out).toContain("\n\n## 核心依赖与协议");
    expect(out).toContain("\n\nMindMesh");
    expect(out).toContain("\n\n1. **One**");
    expect(out).toContain("\n\n2. **Two**");
    expect(out).toContain("\n\n```toml");
    expect(out).toContain("\n\n## 总结");
    const html = marked.parse(out, { async: false }) as string;
    expect(html).toContain("<h2>核心依赖与协议</h2>");
    expect(html).toContain("<li>");
  });

  test("repairs flattened lm studio style markdown", async () => {
    const raw = await Bun.file("/Users/bjsttlp485/.mind-mesh/debug/last-ask-raw.md").text();
    const repaired = repairFlattenedMarkdown(raw);
    expect(repaired.split("\n").length).toBeGreaterThan(5);
    const html = marked.parse(prepareMarkdownForRender(raw), { async: false }) as string;
    expect(html).toMatch(/<h[23]>/);
    expect(html).toContain("<li>");
  });
});
