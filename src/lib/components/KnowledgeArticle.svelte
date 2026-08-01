<script lang="ts">
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

  const preparedBody = $derived(prepareMarkdownForRender(body));
  const headingStructure = $derived(extractMarkdownHeadingStructure(preparedBody));
  const headingIds = $derived(headingStructure.headingIds);
  const headings = $derived(headingStructure.tocHeadings);
  const showToc = $derived(headings.length >= 2);

  // The scroll container is reused across documents, so opening a new one would
  // otherwise inherit the previous document's offset.
  $effect(() => {
    path;
    if (scrollRoot) scrollRoot.scrollTop = 0;
  });
</script>

<div class="knowledge-article-scroll flex-1 overflow-y-auto" bind:this={scrollRoot}>
  <div class="knowledge-article-layout">
    <article class="knowledge-article-body" bind:this={contentRoot}>
      <div class="mb-3 flex items-center gap-2 text-xs text-tr-ink-3">
        <span class="truncate" title={path}>{path}</span>
      </div>
      <MarkdownViewer {body} {repoPath} {onSourceClick} {headingIds} breaks={false} />
    </article>

    {#if showToc}
      <MarkdownArticleToc {headings} scrollRoot={scrollRoot} contentRoot={contentRoot} />
    {/if}
  </div>
</div>

<style>
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
