import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

function isMermaidPreload(dep: string): boolean {
  return /(^|\/)mermaid[-.]/.test(dep) || dep.includes("/mermaid.");
}

export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  build: {
    modulePreload: {
      resolveDependencies(_filename, deps) {
        return deps.filter((dep) => !isMermaidPreload(dep));
      },
    },
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            {
              name: "mermaid",
              test: /node_modules[\\/](mermaid|cytoscape|katex|dagre|@mermaid-js)/,
              priority: 30,
            },
            {
              name: "highlight",
              test: /node_modules[\\/]highlight\.js/,
              priority: 25,
            },
          ],
        },
      },
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // Knowledge assets and indexes are written by the Rust backend during scans/generation.
      ignored: [
        "**/src-tauri/**",
        "**/.terrain/**",
        "**/.litho-agent/**",
        "**/.agents/**",
        "AGENTS.md",
      ],
    },
  },
});
