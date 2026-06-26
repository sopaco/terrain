type Mermaid = typeof import("mermaid").default;

let api: Mermaid | null = null;
let initPromise: Promise<Mermaid> | null = null;

export async function loadMermaid(): Promise<Mermaid> {
  if (api) return api;
  if (!initPromise) {
    initPromise = import("mermaid").then((mod) => {
      const mermaid = mod.default;
      mermaid.initialize({
        startOnLoad: false,
        theme: "dark",
        securityLevel: "loose",
        suppressErrorRendering: true,
        fontFamily: "Inter, ui-sans-serif, system-ui, sans-serif",
      });
      mermaid.parseError = () => {};
      api = mermaid;
      return mermaid;
    });
  }
  return initPromise;
}
