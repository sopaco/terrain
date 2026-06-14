<script lang="ts">
  import mermaid from "mermaid";
  import { marked } from "marked";
  import type { SourceCitation } from "../types";
  import { citationKindForPath } from "../knowledgeDoc";
  import { escapeHtml, hasCompleteMermaidBlocks } from "../mermaid-utils";
  import { linkifySourcesInHtml, prepareMarkdownForRender } from "../markdownSanitize";
  import MermaidLightbox from "./MermaidLightbox.svelte";
  import "../markdown.css";

  interface Props {
    body: string;
    repoPath?: string | null;
    compact?: boolean;
    allowMermaid?: boolean;
    /** Precomputed heading ids (document order) for in-page anchor navigation. */
    headingIds?: string[];
    onSourceClick?: (citation: SourceCitation) => void;
  }

  let {
    body,
    repoPath = null,
    compact = false,
    allowMermaid = true,
    headingIds = [],
    onSourceClick,
  }: Props = $props();

  let container = $state<HTMLDivElement | null>(null);
  let lightboxSvg = $state<string | null>(null);
  let mermaidReady = false;

  const SOURCE_RE =
    /`?([a-zA-Z0-9_./-]+\.(?:rs|ts|tsx|js|jsx|py|go|java|kt|swift|cs|cpp|c|h|md|yaml|yml|toml|json))(?::(\d+)(?:-(\d+))?)?`?/g;

  function linkifySources(text: string): string {
    return text.replace(SOURCE_RE, (match, path: string, start?: string, end?: string) => {
      const s = start ? Number(start) : undefined;
      const e = end ? Number(end) : s;
      const data = encodeURIComponent(JSON.stringify({ path, start: s, end: e }));
      return `<button type="button" class="source-ref" data-ref="${data}">${match}</button>`;
    });
  }

  const preparedBody = $derived(prepareMarkdownForRender(body));

  function createRenderer() {
    let headingIndex = 0;
    const renderer = new marked.Renderer();
    renderer.code = ({ text, lang }) => {
      if (lang === "mermaid") {
        if (!allowMermaid) {
          return `<div class="mermaid-pending"><span class="mermaid-pending-label">Diagram (loading)</span><pre><code>${escapeHtml(text)}</code></pre></div>`;
        }
        return `<div class="mermaid-wrap" data-mermaid-source="${encodeURIComponent(text)}"></div>`;
      }
      const language = lang ? ` class="language-${lang}"` : "";
      return `<pre><code${language}>${escapeHtml(text)}</code></pre>`;
    };
    if (headingIds.length > 0) {
      renderer.heading = ({ text, depth }) => {
        const id = headingIds[headingIndex];
        headingIndex += 1;
        if (id) {
          return `<h${depth} id="${escapeHtml(id)}">${text}</h${depth}>`;
        }
        return `<h${depth}>${text}</h${depth}>`;
      };
    }
    return renderer;
  }

  const html = $derived(
    linkifySourcesInHtml(
      marked.parse(preparedBody, {
        async: false,
        breaks: true,
        gfm: true,
        renderer: createRenderer(),
      }) as string,
      linkifySources,
    ),
  );

  const canRenderMermaid = $derived(allowMermaid && hasCompleteMermaidBlocks(preparedBody));

  async function ensureMermaid() {
    if (mermaidReady) return;
    mermaid.initialize({
      startOnLoad: false,
      theme: "dark",
      securityLevel: "loose",
      suppressErrorRendering: true,
      fontFamily: "Inter, ui-sans-serif, system-ui, sans-serif",
    });
    mermaid.parseError = () => {};
    mermaidReady = true;
  }

  function cleanupMermaidArtifacts() {
    for (const node of document.body.children) {
      if (!(node instanceof HTMLElement)) continue;
      const text = node.textContent ?? "";
      if (
        text.includes("Syntax error in text") ||
        text.includes("mermaid version") && node.id.startsWith("d")
      ) {
        node.remove();
      }
    }
  }

  async function renderMermaidBlocks() {
    if (!container || !canRenderMermaid) return;
    await ensureMermaid();

    const blocks = container.querySelectorAll<HTMLElement>(".mermaid-wrap");
    for (const block of blocks) {
      if (block.dataset.rendered === "true" || block.dataset.rendered === "error") continue;
      const source = decodeURIComponent(block.dataset.mermaidSource ?? "");
      if (!source.trim()) continue;

      const valid = await mermaid.parse(source, { suppressErrors: true });
      if (!valid) {
        block.innerHTML = `<div class="mermaid-fallback"><p class="mermaid-fallback-title">Diagram could not be rendered</p><pre><code>${escapeHtml(source)}</code></pre></div>`;
        block.dataset.rendered = "error";
        continue;
      }

      const id = `mmd-${crypto.randomUUID()}`;
      try {
        const { svg } = await mermaid.render(id, source);
        block.innerHTML = svg;
        block.dataset.rendered = "true";
        block.onclick = () => {
          lightboxSvg = svg;
        };
      } catch {
        block.innerHTML = `<div class="mermaid-fallback"><p class="mermaid-fallback-title">Diagram could not be rendered</p><pre><code>${escapeHtml(source)}</code></pre></div>`;
        block.dataset.rendered = "error";
      }
    }
    cleanupMermaidArtifacts();
  }

  $effect(() => {
    html;
    canRenderMermaid;
    queueMicrotask(() => {
      void renderMermaidBlocks();
    });
  });

  function handleClick(e: MouseEvent) {
    const target = (e.target as HTMLElement).closest(".source-ref") as HTMLElement | null;
    if (!target || !onSourceClick) return;
    e.preventDefault();
    const raw = target.getAttribute("data-ref");
    if (!raw) return;
    const { path, start, end } = JSON.parse(decodeURIComponent(raw)) as {
      path: string;
      start?: number;
      end?: number;
    };
    onSourceClick({
      kind: citationKindForPath(path),
      title: start ? `${path}:${start}` : path,
      path,
      repo_path: repoPath ?? undefined,
      start_line: start,
      end_line: end,
    });
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions a11y_no_static_element_interactions -->
<div
  bind:this={container}
  role="document"
  class={`markdown-body max-w-none ${compact ? "compact" : ""}`}
  onclick={handleClick}
>
  {@html html}
</div>

{#if lightboxSvg}
  <MermaidLightbox svg={lightboxSvg} onclose={() => (lightboxSvg = null)} />
{/if}
