<script lang="ts">
  import { PanelRightClose, PanelRightOpen } from "@lucide/svelte";
  import { tr } from "../i18n";
  import type { TocHeading } from "../markdownToc";
  import { readerLayout, toggleArticleToc } from "../stores/readerLayout.svelte";

  interface Props {
    headings: TocHeading[];
    scrollRoot: HTMLElement | null;
    contentRoot: HTMLElement | null;
  }

  let { headings, scrollRoot, contentRoot }: Props = $props();

  let activeId = $state<string | null>(null);

  const collapsed = $derived(readerLayout.articleTocCollapsed);

  function scrollToHeading(id: string) {
    const el = contentRoot?.querySelector<HTMLElement>(`#${CSS.escape(id)}`);
    if (!el || !scrollRoot) return;
    const rootTop = scrollRoot.getBoundingClientRect().top;
    const offset = el.getBoundingClientRect().top - rootTop + scrollRoot.scrollTop - 12;
    scrollRoot.scrollTo({ top: Math.max(0, offset), behavior: "smooth" });
    activeId = id;
  }

  function updateActiveHeading() {
    if (!scrollRoot || !contentRoot || headings.length === 0) return;

    const rootTop = scrollRoot.getBoundingClientRect().top;
    const marker = 88;
    let current: string | null = null;

    for (const heading of headings) {
      const el = contentRoot.querySelector<HTMLElement>(`#${CSS.escape(heading.id)}`);
      if (!el) continue;
      const top = el.getBoundingClientRect().top - rootTop;
      if (top <= marker) current = heading.id;
    }

    activeId = current ?? headings[0]?.id ?? null;
  }

  $effect(() => {
    headings;
    scrollRoot;
    contentRoot;

    if (collapsed) return;
    if (!scrollRoot || !contentRoot) return;

    const onScroll = () => updateActiveHeading();
    scrollRoot.addEventListener("scroll", onScroll, { passive: true });
    queueMicrotask(updateActiveHeading);

    return () => scrollRoot.removeEventListener("scroll", onScroll);
  });

  function indent(level: number): string {
    return `${Math.max(0, level - 1) * 0.625}rem`;
  }
</script>

<aside class={`article-toc ${collapsed ? "collapsed" : ""}`} aria-label={tr("knowledge.toc.ariaLabel")}>
  {#if collapsed}
    <button
      type="button"
      class="article-toc-rail"
      onclick={toggleArticleToc}
      aria-expanded="false"
      title={tr("knowledge.toc.expand")}
    >
      <PanelRightOpen size={14} strokeWidth={2} aria-hidden="true" />
      <span class="article-toc-rail-label">{tr("knowledge.toc.title")}</span>
    </button>
  {:else}
    <div class="article-toc-panel">
      <div class="article-toc-header">
        <span class="article-toc-title">{tr("knowledge.toc.title")}</span>
        <span class="article-toc-count">{headings.length}</span>
        <button
          type="button"
          class="article-toc-collapse"
          onclick={toggleArticleToc}
          aria-expanded="true"
          aria-label={tr("knowledge.toc.collapse")}
          title={tr("knowledge.toc.collapse")}
        >
          <PanelRightClose size={14} strokeWidth={2} aria-hidden="true" />
        </button>
      </div>

      <nav class="article-toc-nav">
        <ul class="article-toc-list">
          {#each headings as heading (heading.id)}
            <li class="article-toc-item" style={`--indent: ${indent(heading.level)}`}>
              <button
                type="button"
                class={`article-toc-link ${activeId === heading.id ? "active" : ""}`}
                onclick={() => scrollToHeading(heading.id)}
                title={heading.text}
              >
                {heading.text}
              </button>
            </li>
          {/each}
        </ul>
      </nav>
    </div>
  {/if}
</aside>

<style>
  .article-toc {
    width: 13rem;
    flex-shrink: 0;
    align-self: flex-start;
    position: sticky;
    /* Matches the layout's top padding so it does not jump when it starts sticking. */
    top: 2rem;
    max-height: calc(100vh - 12rem);
    display: flex;
    flex-direction: column;
  }

  /*
   * `100vh` overshoots by the toolbar above and the ask bar below, letting a long
   * outline run off the bottom. `cqh` is the real height of the scrollport, which
   * KnowledgeArticle declares as a size container.
   */
  @supports (height: 1cqh) {
    .article-toc {
      max-height: calc(100cqh - 4rem);
    }
  }

  .article-toc.collapsed {
    width: 2.25rem;
  }

  .article-toc-rail {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    padding: 0.5rem 0;
    border: none;
    background: transparent;
    color: var(--color-tr-ink-3);
    cursor: pointer;
    border-radius: var(--radius-lg);
    transition: background 0.15s, color 0.15s;
  }

  .article-toc-rail:hover {
    background: var(--color-tr-elevated);
    color: var(--color-tr-ink);
  }

  .article-toc-rail-label {
    font-size: 0.625rem;
    letter-spacing: 0.08em;
    writing-mode: vertical-rl;
    user-select: none;
  }

  .article-toc-panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-radius: var(--radius-xl);
    border: 1px solid var(--color-tr-border);
    background: var(--color-tr-elevated);
    overflow: hidden;
  }

  .article-toc-header {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    width: 100%;
    padding: 0.625rem 0.75rem;
    color: var(--color-tr-ink-2);
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.02em;
  }

  .article-toc-title {
    flex: 1;
    text-align: left;
  }

  .article-toc-count {
    font-size: 0.625rem;
    font-weight: 500;
    color: var(--color-tr-ink-3);
    background: var(--color-tr-raised);
    border-radius: 9999px;
    padding: 0.1rem 0.4rem;
  }

  .article-toc-collapse {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    margin-right: -0.25rem;
    padding: 0.25rem;
    border: none;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--color-tr-ink-3);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .article-toc-collapse:hover {
    background: var(--color-tr-raised);
    color: var(--color-tr-ink);
  }

  .article-toc-nav {
    overflow-y: auto;
    padding: 0.25rem 0 0.5rem;
  }

  .article-toc-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .article-toc-item {
    margin: 0;
  }

  .article-toc-link {
    display: block;
    width: 100%;
    text-align: left;
    border: none;
    background: transparent;
    color: var(--color-tr-ink-3);
    font-size: 0.75rem;
    line-height: 1.45;
    padding: 0.3rem 0.75rem 0.3rem calc(0.75rem + var(--indent, 0rem));
    border-left: 2px solid transparent;
    cursor: pointer;
    transition: color 0.15s, background 0.15s, border-color 0.15s;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .article-toc-link:hover {
    color: var(--color-tr-ink-2);
    background: var(--color-tr-raised);
  }

  .article-toc-link.active {
    color: var(--color-tr-accent);
    border-left-color: var(--color-tr-accent);
    background: var(--color-tr-accent-soft);
  }

  .article-toc-link.active:hover {
    color: var(--color-tr-accent);
    background: var(--color-tr-accent-soft);
  }
</style>
