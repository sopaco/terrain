import { describe, expect, it } from "vitest";
import {
  buildSourceCitation,
  linkifySourceSegments,
  parseSourceRef,
} from "./sourceRef";

describe("parseSourceRef", () => {
  it("parses bare file paths", () => {
    expect(parseSourceRef("crates/foo/src/lib.rs")).toEqual({
      path: "crates/foo/src/lib.rs",
      start: undefined,
      end: undefined,
    });
  });

  it("parses path with line range", () => {
    expect(parseSourceRef("src/App.svelte:120-140")).toEqual({
      path: "src/App.svelte",
      start: 120,
      end: 140,
    });
  });

  it("parses path with single line", () => {
    expect(parseSourceRef("schema.rs:180")).toEqual({
      path: "schema.rs",
      start: 180,
      end: 180,
    });
  });

  it("parses backtick-wrapped paths", () => {
    expect(parseSourceRef("`crates/core/src/main.rs:42`")).toEqual({
      path: "crates/core/src/main.rs",
      start: 42,
      end: 42,
    });
  });

  it("rejects non-path inline code", () => {
    expect(parseSourceRef("some_function()")).toBeNull();
    expect(parseSourceRef("HTTP 404")).toBeNull();
  });
});

describe("linkifySourceSegments", () => {
  it("wraps path segments in prose", () => {
    const out = linkifySourceSegments(
      "See crates/foo/bar.rs:10 for details.",
      (match) => `<link>${match}</link>`,
    );
    expect(out).toBe("See <link>crates/foo/bar.rs:10</link> for details.");
  });
});

describe("buildSourceCitation", () => {
  it("classifies rust paths as source_code", () => {
    const c = buildSourceCitation({ path: "src/lib.rs", start: 5 }, "/repo");
    expect(c.kind).toBe("source_code");
    expect(c.title).toBe("src/lib.rs:5");
    expect(c.repo_path).toBe("/repo");
  });

  it("classifies human docs", () => {
    const c = buildSourceCitation({ path: "human/1.概述.md" });
    expect(c.kind).toBe("human_doc");
  });
});
