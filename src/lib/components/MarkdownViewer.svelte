<script lang="ts">
  import { marked } from "marked";
  import { loadMermaid } from "../mermaidLoader";
  import type { SourceCitation } from "../types";
  import { escapeHtml, hasCompleteMermaidBlocks } from "../mermaid-utils";
  import { linkifySourcesInHtml, prepareMarkdownForRender } from "../markdownSanitize";
  import {
    buildSourceCitation,
    linkifySourceSegments,
    parseSourceRef,
    sourceRefDataAttr,
  } from "../sourceRef";
  import MermaidLightbox from "./MermaidLightbox.svelte";
  import "../markdown.css";

  interface Props {
    body: string;
    repoPath?: string | null;
    compact?: boolean;
    allowMermaid?: boolean;
    /**
     * Render a single newline as `<br>`. Right for chat, where models emit soft
     * wraps mid-paragraph; wrong for authored documents, where it turns a
     * hard-wrapped paragraph into a run of ragged short lines.
     */
    breaks?: boolean;
    /** Precomputed heading ids (document order) for in-page anchor navigation. */
    headingIds?: string[];
    onSourceClick?: (citation: SourceCitation) => void;
  }

  let {
    body,
    repoPath = null,
    compact = false,
    allowMermaid = true,
    breaks = true,
    headingIds = [],
    onSourceClick,
  }: Props = $props();

  let container = $state<HTMLDivElement | null>(null);
  let lightboxSvg = $state<string | null>(null);

  function sourceRefButton(match: string, parsed: ReturnType<typeof parseSourceRef>) {
    if (!parsed) return match;
    const data = sourceRefDataAttr(parsed);
    return `<button type="button" class="source-ref" data-ref="${data}">${escapeHtml(match)}</button>`;
  }

  function linkifySources(text: string): string {
    return linkifySourceSegments(text, (match, parsed) => sourceRefButton(match, parsed));
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
    renderer.codespan = ({ text }) => {
      const parsed = parseSourceRef(text);
      if (parsed && onSourceClick) {
        const data = sourceRefDataAttr(parsed);
        return `<button type="button" class="source-ref source-ref-inline" data-ref="${data}"><code>${escapeHtml(text)}</code></button>`;
      }
      return `<code>${escapeHtml(text)}</code>`;
    };
    // A table must scroll inside its own box; making the table itself
    // `display: block` would drop `width: 100%` and collapse the column widths.
    const baseTable = renderer.table.bind(renderer);
    renderer.table = (token) => `<div class="markdown-table-wrap">${baseTable(token)}</div>`;
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
        breaks,
        gfm: true,
        renderer: createRenderer(),
      }) as string,
      linkifySources,
    ),
  );

  const canRenderMermaid = $derived(allowMermaid && hasCompleteMermaidBlocks(preparedBody));

  function cleanupMermaidArtifacts() {
    for (const node of document.body.children) {
      if (!(node instanceof HTMLElement)) continue;
      const text = node.textContent ?? "";
      if (
        text.includes("Syntax error in text") ||
        (text.includes("mermaid version") && node.id.startsWith("d"))
      ) {
        node.remove();
      }
    }
  }

  async function renderMermaidBlocks() {
    if (!container || !canRenderMermaid) return;
    const mermaid = await loadMermaid();

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
    e.stopPropagation();
    const raw = target.getAttribute("data-ref");
    if (!raw) return;
    const parsed = JSON.parse(decodeURIComponent(raw)) as {
      path: string;
      start?: number;
      end?: number;
    };
    onSourceClick(buildSourceCitation(parsed, repoPath));
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  bind:this={container}
  class={`markdown-body max-w-none ${compact ? "compact" : ""}`}
  onclick={handleClick}
>
  {@html html}
</div>

{#if lightboxSvg}
  <MermaidLightbox svg={lightboxSvg} onclose={() => (lightboxSvg = null)} />
{/if}
