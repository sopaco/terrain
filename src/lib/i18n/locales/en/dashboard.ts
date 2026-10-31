/** Fleet dashboard panel (cross-project metrics, batch updates, multi-add). */
export default {
  title: "Fleet Dashboard",
  descLine1: "Graph metrics and batch updates across every registered project.",
  refresh: "Refresh",
  totals: {
    projects: "Projects",
    graphs: "With graph",
    nodes: "Nodes",
    links: "Links",
    stale: "Stale graphs",
    costToday: "Cost today",
  },
  binary: {
    missing: "Graphify CLI not found — batch updates are disabled.",
    available: "Graphify CLI found at",
  },
  actions: {
    add: "Add projects",
    adding: "Adding…",
    batch: "Update all graphs",
    batching: "Updating fleet…",
    updateRow: "Update",
    openFolder: "Open",
  },
  table: {
    project: "Project",
    status: "Terrain",
    graph: "Graph",
    staleness: "Graph state",
    costToday: "Cost today",
    lastBatch: "Last batch",
    missingAssets: { count: "{count} missing" },
  },
  status: {
    ready: "Ready",
    partial: "Incomplete",
    stale: "Needs repair",
  },
  graph: {
    fresh: "Up to date",
    stale: "Stale",
    unknown: "No commit data",
    none: "—",
  },
  batch: {
    lastRun: "Last update",
    never: "Never updated",
    duration: "{secs}" + "s",
    error: "Error",
  },
  add: {
    resultAdded: "Added {count} project(s)",
    resultErrors: "Failed: {errors}",
    resultScanWarn: "added; scan incomplete for {count}",
    nonePicked: "No folders selected.",
  },
  empty: {
    title: "No projects registered",
    hint: "Add repositories to see fleet-wide graph metrics here.",
  },
  batchDone: "Fleet update finished: {succeeded} ok, {failed} failed",
  delete: {
    confirmTitle: "Delete project",
    confirmBody:
      'Remove "{name}" from the workspace registry? Its .terrain/ knowledge assets on disk are not deleted.',
  },
  menu: {
    label: "Row actions",
    update: "Update graph",
    open: "Open folder",
    remove: "Delete project",
  },
  charts: {
    commits: "Commit activity",
    commitsTotal: "{count} commits / 12w",
    collab: "Cross-project collaborations (inferred)",
    collabHint: "Inferred from shared git contributors",
    sharedAuthors: "{count} shared authors",
    noCollab: "No shared contributors found between projects",
    coverage: "Documentation coverage",
  },
  docs: {
    agentContext: "Agent context",
    humanDocs: "Human docs",
    repomix: "Repomix",
    graph: "Graph",
    wiki: "Wiki",
    vault: "Vault",
    present: "present",
    missing: "missing",
  },
  vault: {
    banner:
      "No Obsidian vault found at ~/vault — vault integration is off for graphify projects.",
    bannerHint:
      "Create ~/vault (or run the memory-setup installer) and re-run a project update to link the graph.",
    rowHint: "Vault link not configured — run Update on this project.",
  },
} as const;