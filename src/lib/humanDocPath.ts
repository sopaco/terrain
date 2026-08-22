import type { HumanDocEntry } from "./types";

/** Path within the human-knowledge tree (strips section prefixes). */
export function humanDocTreePath(doc: HumanDocEntry): string {
  const rel = doc.relative_path.replace(/\\/g, "/");
  const section = doc.section ?? "human";
  if (section === "agent") {
    const idx = rel.indexOf("/agent/");
    return idx >= 0 ? rel.slice(idx + "/agent/".length) : rel.replace(/^agent\//, "");
  }
  if (section === "structured") {
    return rel;
  }
  const humanInfix = rel.indexOf("/human/");
  if (humanInfix >= 0) return rel.slice(humanInfix + "/human/".length);
  return rel.replace(/^human\//, "");
}

/** Litho overview doc: `1.概述.md` (zh) or `1.Overview.md` (en). */
export function findHumanOverviewDoc(
  docs: HumanDocEntry[],
): HumanDocEntry | undefined {
  return docs.find(
    (d) => d.section === "human" && /^1\..+\.md$/i.test(humanDocTreePath(d)),
  );
}
