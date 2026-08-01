/**
 * Collapse state for the knowledge reader's two navigation rails.
 * Collapsing either one hands its width back to the article column.
 */

const DOC_TREE_KEY = "terrain.reader.docTreeCollapsed";
const ARTICLE_TOC_KEY = "terrain.reader.articleTocCollapsed";

function readCollapsed(key: string): boolean {
  if (typeof localStorage === "undefined") return false;
  return localStorage.getItem(key) === "true";
}

function persist(key: string, collapsed: boolean) {
  if (typeof localStorage !== "undefined") {
    localStorage.setItem(key, String(collapsed));
  }
}

export const readerLayout = $state({
  docTreeCollapsed: readCollapsed(DOC_TREE_KEY),
  articleTocCollapsed: readCollapsed(ARTICLE_TOC_KEY),
});

export function toggleDocTree() {
  readerLayout.docTreeCollapsed = !readerLayout.docTreeCollapsed;
  persist(DOC_TREE_KEY, readerLayout.docTreeCollapsed);
}

export function toggleArticleToc() {
  readerLayout.articleTocCollapsed = !readerLayout.articleTocCollapsed;
  persist(ARTICLE_TOC_KEY, readerLayout.articleTocCollapsed);
}
