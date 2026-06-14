import { describe, expect, test } from "bun:test";
import {
  extractMarkdownHeadings,
  slugifyHeading,
  stripMarkdownInline,
} from "./markdownToc";

describe("stripMarkdownInline", () => {
  test("removes inline formatting", () => {
    expect(stripMarkdownInline("**Bold** and `code`")).toBe("Bold and code");
    expect(stripMarkdownInline("[Link](https://example.com)")).toBe("Link");
  });
});

describe("slugifyHeading", () => {
  test("creates ascii slugs", () => {
    expect(slugifyHeading("Hello World")).toBe("hello-world");
  });

  test("keeps cjk characters", () => {
    expect(slugifyHeading("架构概览")).toBe("架构概览");
  });
});

describe("extractMarkdownHeadings", () => {
  test("extracts headings with levels and ids", () => {
    const md = `# Title

## Overview

### Details

## Overview
`;
    const headings = extractMarkdownHeadings(md);
    expect(headings).toEqual([
      { level: 1, text: "Title", id: "title" },
      { level: 2, text: "Overview", id: "overview" },
      { level: 3, text: "Details", id: "details" },
      { level: 2, text: "Overview", id: "overview-1" },
    ]);
  });

  test("skips headings inside fenced code", () => {
    const md = `## Real

\`\`\`md
## Fake
\`\`\`

## Also Real
`;
    const headings = extractMarkdownHeadings(md);
    expect(headings.map((h) => h.text)).toEqual(["Real", "Also Real"]);
  });

  test("strips inline markdown from display text", () => {
    const md = "## **Bold** section";
    const headings = extractMarkdownHeadings(md);
    expect(headings[0]?.text).toBe("Bold section");
    expect(headings[0]?.id).toBe("bold-section");
  });
});
