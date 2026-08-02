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
  import { copyTextToClipboard } from "../clipboard";
  import MermaidLightbox from "./MermaidLightbox.svelte";
  import "../syntax-tokens.css";
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
    /**
     * Syntax-highlight fenced code. Turn off while streaming: the rendered HTML
     * is rebuilt on every chunk, so every block would be re-highlighted.
     */
    highlight?: boolean;
    /** Precomputed heading ids (document order) for in-page anchor navigation. */
    headingIds?: string[];
    /** Softer typography for long-form reading (knowledge articles). */
    reading?: boolean;
    onSourceClick?: (citation: SourceCitation) => void;
  }

  let {
    body,
    repoPath = null,
    compact = false,
    allowMermaid = true,
    breaks = true,
    highlight = true,
    headingIds = [],
    reading = false,
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
      const label = (lang ?? "").trim();
      const langClass = label ? ` class="language-${escapeHtml(label)}"` : "";
      const langAttr = label ? ` data-lang="${escapeHtml(label)}"` : "";
      return (
        `<div class="code-block tr-syntax"${langAttr}>` +
        `<div class="code-block-bar">` +
        `<span class="code-block-lang">${escapeHtml(label || "text")}</span>` +
        `<button type="button" class="code-copy" aria-label="复制代码">复制</button>` +
        `</div>` +
        `<pre><code${langClass}>${escapeHtml(text)}</code></pre>` +
        `</div>`
      );
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
      // `token.text` is raw markdown — emitting it would show `` `code` `` and
      // `**bold**` inside headings verbatim, so parse the inline tokens instead.
      renderer.heading = function ({ tokens, depth }) {
        const id = headingIds[headingIndex];
        headingIndex += 1;
        const content = this.parser.parseInline(tokens);
        if (id) {
          return `<h${depth} id="${escapeHtml(id)}">${content}</h${depth}>`;
        }
        return `<h${depth}>${content}</h${depth}>`;
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

  async function highlightCodeBlocks() {
    if (!container || !highlight) return;
    const blocks = container.querySelectorAll<HTMLElement>(".code-block code");
    if (blocks.length === 0) return;

    const { highlightFencedCode } = await import("../highlightSetup");
    for (const block of blocks) {
      if (block.dataset.highlighted === "true") continue;
      // Set before awaiting nothing further, so a re-render mid-loop cannot double-apply.
      block.dataset.highlighted = "true";
      const raw = block.textContent ?? "";
      if (!raw.trim()) continue;
      const lang = block.closest<HTMLElement>(".code-block")?.dataset.lang;
      block.innerHTML = highlightFencedCode(raw, lang);
      block.classList.add("hljs");
    }
  }

  async function copyCodeBlock(button: HTMLElement) {
    const code = button.closest(".code-block")?.querySelector("code");
    const text = code?.textContent ?? "";
    if (!text) return;

    const original = button.textContent;
    try {
      await copyTextToClipboard(text);
      button.textContent = "已复制";
    } catch {
      button.textContent = "复制失败";
    }
    setTimeout(() => {
      // The block may have been re-rendered while the label was swapped.
      if (button.isConnected) button.textContent = original;
    }, 1800);
  }

  $effect(() => {
    html;
    canRenderMermaid;
    highlight;
    queueMicrotask(() => {
      void renderMermaidBlocks();
      void highlightCodeBlocks();
    });
  });

  function handleClick(e: MouseEvent) {
    const copyButton = (e.target as HTMLElement).closest(".code-copy") as HTMLElement | null;
    if (copyButton) {
      e.preventDefault();
      e.stopPropagation();
      void copyCodeBlock(copyButton);
      return;
    }

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
  class={`markdown-body max-w-none ${compact ? "compact" : ""} ${reading ? "reading" : ""}`}
  onclick={handleClick}
>
  {@html html}
</div>

{#if lightboxSvg}
  <MermaidLightbox svg={lightboxSvg} onclose={() => (lightboxSvg = null)} />
{/if}
