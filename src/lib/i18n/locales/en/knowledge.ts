/** Knowledge browsing: document tree, article reader, TOC, markdown affordances. */
export default {
  tree: {
    title: "Documents",
    count: "{count} docs",
    countFiltered: "{visible}/{total}",
    collapse: "Collapse document tree",
    filterPlaceholder: "Filter documents…",
    filterAria: "Filter documents",
    clearFilter: "Clear filter",
    sectionStructured: "Structured Index",
    folderModules: "Modules",
    folderInterfaces: "Interfaces",
    folderRoutes: "Routes",
    folderEvents: "Events",
    folderDeepExploration: "Deep Exploration",
    noMatchPrefix: 'No documents match "',
    noMatchSuffix: '".',
    emptyPrefix: "No {term} yet. Click ",
    emptySuffix: " in the toolbar.",
  },
  article: {
    pathAria: "Document path",
    backToTop: "Back to top",
    top: "Top",
  },
  toc: {
    ariaLabel: "Table of contents",
    title: "On this page",
    expand: "Expand page outline",
    collapse: "Collapse page outline",
  },
  markdown: {
    copyCode: "Copy code",
  },
} as const;
