/** Misc UI copy: help, nav tabs, project selector, progress bar, mermaid lightbox. */
export default {
  help: {
    title: "Glossary",
    subtitle: "Quick reference for common Terrain concepts",
    button: "Help",
    footerKnowledge: "Knowledge bases live in each repo's",
    footerRegistry: "directory; the project list is registered in",
    footerEnd: ".",
  },
  progress: {
    generating: "Generating knowledge assets",
  },
  projects: {
    empty: "No projects yet — add a repository.",
    add: "Add & initialize repository",
    adding: "Adding and initializing…",
    openFolder: "Open repository folder",
    remove: "Remove from list",
    removeAria: "Remove {label} from list",
    removeConfirm:
      'Remove "{label}" from the list?\n\nThis only removes the Terrain registration; the repository and its .terrain/ knowledge assets are not deleted.',
    statusStale: "Needs repair",
    statusPartial: "Incomplete",
    repairStale:
      "The repository's `.terrain` is missing or corrupted ({path}); rescan to regenerate knowledge assets.",
    repairMissing: "Not ready: {assets}.",
    repairPartial: "Some knowledge assets are not ready.",
    repairSuffix: "{missing} ({path})",
    assetJoin: ", ",
  },
  nav: {
    overview: "Overview",
    overviewTitle: "Project overview",
    knowledge: "Knowledge",
    env: "Environment",
    sddTitle: "SDD workflow: spec-driven development",
  },
  mermaid: {
    title: "Mermaid diagram",
    zoomIn: "Zoom in (+)",
    zoomOut: "Zoom out (-)",
    resetView: "Reset view (0)",
    copyImage: "Copy image",
    copying: "Copying…",
    copiedImage: "Image copied to clipboard",
    copiedSvg: "SVG source copied to clipboard",
    copyFailed: "Copy failed: {error}",
  },
} as const;
