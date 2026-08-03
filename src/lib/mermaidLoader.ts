type Mermaid = typeof import("mermaid").default;

/** Mirrors design tokens in `src/app.css` (`--color-tr-*`). */
const TERRAIN = {
  page: "#0a0d10",
  surface: "#12161b",
  elevated: "#181d23",
  raised: "#1e2530",
  border: "#2a323c",
  borderStrong: "#3d4a56",
  ink: "#e8ecef",
  inkReading: "#c8d0d6",
  ink2: "#a2acb5",
  ink3: "#6c7680",
  accent: "#1f8f84",
  accentHover: "#2aa89b",
  accentTint: "#1a2e2c",
  onAccent: "#f3fbfa",
  good: "#5fb37e",
  watch: "#d9a441",
} as const;

const FONT_FAMILY = "Inter, ui-sans-serif, system-ui, sans-serif";

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
        fontFamily: FONT_FAMILY,
        htmlLabels: true,
        themeVariables: {
          background: "transparent",
          primaryColor: TERRAIN.raised,
          primaryBorderColor: TERRAIN.accent,
          primaryTextColor: TERRAIN.ink,
          secondaryColor: TERRAIN.elevated,
          secondaryBorderColor: TERRAIN.borderStrong,
          secondaryTextColor: TERRAIN.inkReading,
          tertiaryColor: TERRAIN.accentTint,
          tertiaryBorderColor: TERRAIN.accentHover,
          tertiaryTextColor: TERRAIN.inkReading,
          lineColor: TERRAIN.ink3,
          textColor: TERRAIN.inkReading,
          mainBkg: TERRAIN.raised,
          nodeBorder: TERRAIN.accent,
          clusterBkg: TERRAIN.surface,
          clusterBorder: TERRAIN.border,
          titleColor: TERRAIN.ink,
          edgeLabelBackground: TERRAIN.elevated,
          defaultLinkColor: TERRAIN.ink3,
          arrowheadColor: TERRAIN.ink2,
          labelBackground: TERRAIN.elevated,
          actorBkg: TERRAIN.raised,
          actorBorder: TERRAIN.accent,
          actorTextColor: TERRAIN.ink,
          signalColor: TERRAIN.ink3,
          signalTextColor: TERRAIN.inkReading,
          noteBkgColor: TERRAIN.elevated,
          noteBorderColor: TERRAIN.accent,
          noteTextColor: TERRAIN.inkReading,
          activationBkgColor: TERRAIN.accentTint,
          activationBorderColor: TERRAIN.accent,
          gridColor: TERRAIN.border,
          taskBkgColor: TERRAIN.raised,
          taskBorderColor: TERRAIN.accent,
          taskTextColor: TERRAIN.inkReading,
          activeTaskBkgColor: TERRAIN.accentTint,
          doneTaskBkgColor: TERRAIN.good,
          critBkgColor: TERRAIN.watch,
          labelTextColor: TERRAIN.inkReading,
          errorBkgColor: "#3a1f1a",
          errorTextColor: TERRAIN.ink,
          useGradient: false,
          radius: 4,
        },
        flowchart: {
          curve: "linear",
          nodeSpacing: 60,
          rankSpacing: 100,
          padding: 20,
          useMaxWidth: true,
        },
        sequence: {
          rightAngles: true,
          useMaxWidth: true,
        },
      });
      mermaid.parseError = () => {};
      api = mermaid;
      return mermaid;
    });
  }
  return initPromise;
}
