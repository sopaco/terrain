<script lang="ts">
  import { ArrowUp } from "@lucide/svelte";
  import MarkdownViewer from "./MarkdownViewer.svelte";
  import MarkdownArticleToc from "./MarkdownArticleToc.svelte";
  import type { SourceCitation } from "../types";
  import { prepareMarkdownForRender } from "../markdownSanitize";
  import { extractMarkdownHeadingStructure } from "../markdownToc";

  interface Props {
    body: string;
    path: string;
    repoPath?: string | null;
    onSourceClick?: (citation: SourceCitation) => void;
  }

  let { body, path, repoPath = null, onSourceClick }: Props = $props();

  let scrollRoot = $state<HTMLDivElement | null>(null);
  let contentRoot = $state<HTMLElement | null>(null);
  let scrolled = $state(false);

  const preparedBody = $derived(prepareMarkdownForRender(body));
  const headingStructure = $derived(extractMarkdownHeadingStructure(preparedBody));
  const headingIds = $derived(headingStructure.headingIds);
  const headings = $derived(headingStructure.tocHeadings);
  const showToc = $derived(headings.length >= 2);

  /** Path segments below `.terrain/`, without the `.md` extension on the last one. */
  const crumbs = $derived.by(() => {
    const rel = path.replace(/\\/g, "/");
    const idx = rel.indexOf(".terrain/");
    const trimmed = idx >= 0 ? rel.slice(idx + ".terrain/".length) : rel;
    const parts = trimmed.split("/").filter(Boolean);
    const tail = parts.length > 3 ? parts.slice(-3) : parts;
    return tail.map((part, i) =>
      i === tail.length - 1 ? part.replace(/\.md$/i, "") : part,
    );
  });

  /** The document's own first heading reads better than a slugged filename. */
  const title = $derived(
    headings[0]?.text ?? crumbs[crumbs.length - 1] ?? path,
  );

  // The scroll container is reused across documents, so opening a new one would
  // otherwise inherit the previous document's offset.
  $effect(() => {
    path;
    if (scrollRoot) scrollRoot.scrollTop = 0;
    scrolled = false;
  });

  function onScroll() {
    if (scrollRoot) scrolled = scrollRoot.scrollTop > 240;
  }

  function scrollToTop() {
    scrollRoot?.scrollTo({ top: 0, behavior: "smooth" });
  }
</script>

<div class="knowledge-article-root">
  <header class="knowledge-article-header">
    <div class="knowledge-article-header-inner">
      <div class="knowledge-article-heading">
        <nav class="knowledge-article-crumbs" aria-label="文档路径" title={path}>
          {#each crumbs as crumb, i}
            {#if i > 0}<span class="knowledge-article-crumb-sep" aria-hidden="true">/</span>{/if}
            <span class="knowledge-article-crumb" class:is-last={i === crumbs.length - 1}
              >{crumb}</span
            >
          {/each}
        </nav>
        <h1 class="knowledge-article-title" title={title}>{title}</h1>
      </div>

      {#if scrolled}
        <button
          type="button"
          class="knowledge-article-top"
          onclick={scrollToTop}
          aria-label="回到顶部"
          title="回到顶部"
        >
          <ArrowUp size={13} strokeWidth={2} aria-hidden="true" />
          顶部
        </button>
      {/if}
    </div>
  </header>

  <div
    class="knowledge-article-scroll"
    bind:this={scrollRoot}
    onscroll={onScroll}
  >
    <div class="knowledge-article-layout">
      <article class="knowledge-article-body" bind:this={contentRoot}>
        <MarkdownViewer {body} {repoPath} {onSourceClick} {headingIds} breaks={false} />
      </article>

      {#if showToc}
        <MarkdownArticleToc {headings} scrollRoot={scrollRoot} contentRoot={contentRoot} />
      {/if}
    </div>
  </div>
</div>

<style>
  .knowledge-article-root {
    display: flex;
    min-height: 0;
    flex: 1;
    flex-direction: column;
  }

  /*
   * Kept outside the scrollport rather than `position: sticky` inside it: the
   * TOC is already sticky in there, and two stacked sticky elements would need
   * their offsets kept in sync by hand.
   */
  .knowledge-article-header {
    flex-shrink: 0;
    border-bottom: 1px solid var(--color-tr-border);
    background: var(--color-tr-surface);
  }

  .knowledge-article-header-inner {
    display: flex;
    align-items: center;
    gap: 1rem;
    max-width: 88rem;
    margin: 0 auto;
    padding: 0.625rem 2rem;
  }

  .knowledge-article-heading {
    min-width: 0;
    flex: 1;
  }

  .knowledge-article-crumbs {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.6875rem;
    color: var(--color-tr-ink-3);
  }

  .knowledge-article-crumb {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .knowledge-article-crumb.is-last {
    color: var(--color-tr-ink-2);
  }

  .knowledge-article-crumb-sep {
    color: var(--color-tr-ink-4);
  }

  .knowledge-article-title {
    margin: 0.0625rem 0 0;
    font-size: 0.875rem;
    font-weight: 600;
    line-height: 1.35;
    color: var(--color-tr-ink);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .knowledge-article-top {
    display: inline-flex;
    flex-shrink: 0;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--color-tr-border-strong);
    border-radius: var(--radius-lg);
    background: transparent;
    color: var(--color-tr-ink-2);
    font-size: 0.6875rem;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .knowledge-article-top:hover {
    background: var(--color-tr-elevated);
    color: var(--color-tr-ink);
  }

  /*
   * A size container so the sticky TOC can bound itself with `cqh` — its height
   * comes from `flex: 1`, never from its contents, so size containment is safe.
   */
  .knowledge-article-scroll {
    flex: 1;
    overflow-y: auto;
    container-type: size;
  }

  .knowledge-article-layout {
    display: flex;
    gap: 2rem;
    align-items: flex-start;
    max-width: 88rem;
    margin: 0 auto;
    padding: 2rem;
  }

  /*
   * `flex: 1` claims whatever either rail gives back when collapsed; the cap
   * keeps line length sane on wide windows, and `margin-inline: auto` centers
   * the column in the released space instead of leaving a gap on the far side.
   */
  .knowledge-article-body {
    min-width: 0;
    flex: 1;
    max-width: 72rem;
    margin-inline: auto;
  }

  @media (max-width: 960px) {
    .knowledge-article-layout {
      flex-direction: column;
      gap: 1rem;
    }
  }
</style>
