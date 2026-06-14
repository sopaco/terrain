<script lang="ts">
  import type { TocHeading } from "../markdownToc";

  interface Props {
    headings: TocHeading[];
    scrollRoot: HTMLElement | null;
    contentRoot: HTMLElement | null;
  }

  let { headings, scrollRoot, contentRoot }: Props = $props();

  const STORAGE_KEY = "mind-mesh:article-toc-expanded";

  let expanded = $state(readExpandedPreference());
  let activeId = $state<string | null>(null);

  function readExpandedPreference(): boolean {
    if (typeof localStorage === "undefined") return true;
    return localStorage.getItem(STORAGE_KEY) !== "false";
  }

  function toggleExpanded() {
    expanded = !expanded;
    localStorage.setItem(STORAGE_KEY, String(expanded));
  }

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

<aside class="article-toc" aria-label="文章目录">
  <div class="article-toc-panel">
    <button
      type="button"
      class="article-toc-header"
      onclick={toggleExpanded}
      aria-expanded={expanded}
      title={expanded ? "收起目录" : "展开目录"}
    >
      <span class="article-toc-title">本页目录</span>
      <span class="article-toc-count">{headings.length}</span>
      <svg
        class={`article-toc-chevron ${expanded ? "expanded" : ""}`}
        viewBox="0 0 20 20"
        fill="currentColor"
        aria-hidden="true"
      >
        <path
          fill-rule="evenodd"
          d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.94a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z"
          clip-rule="evenodd"
        />
      </svg>
    </button>

    {#if expanded}
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
    {/if}
  </div>
</aside>

<style>
  .article-toc {
    width: 13rem;
    flex-shrink: 0;
    align-self: flex-start;
    position: sticky;
    top: 0;
    max-height: calc(100vh - 12rem);
    display: flex;
    flex-direction: column;
  }

  .article-toc-panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-radius: 0.75rem;
    border: 1px solid rgba(255, 255, 255, 0.08);
    background: rgba(255, 255, 255, 0.03);
    overflow: hidden;
  }

  .article-toc-header {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    width: 100%;
    padding: 0.625rem 0.75rem;
    border: none;
    background: rgba(255, 255, 255, 0.02);
    color: rgba(255, 255, 255, 0.75);
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.02em;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .article-toc-header:hover {
    background: rgba(255, 255, 255, 0.06);
    color: rgba(255, 255, 255, 0.92);
  }

  .article-toc-title {
    flex: 1;
    text-align: left;
  }

  .article-toc-count {
    font-size: 0.625rem;
    font-weight: 500;
    color: rgba(255, 255, 255, 0.35);
    background: rgba(255, 255, 255, 0.06);
    border-radius: 9999px;
    padding: 0.1rem 0.4rem;
  }

  .article-toc-chevron {
    width: 1rem;
    height: 1rem;
    color: rgba(255, 255, 255, 0.45);
    transition: transform 0.2s ease;
    flex-shrink: 0;
  }

  .article-toc-chevron.expanded {
    transform: rotate(180deg);
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
    color: rgba(255, 255, 255, 0.5);
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
    color: rgba(255, 255, 255, 0.82);
    background: rgba(255, 255, 255, 0.04);
  }

  .article-toc-link.active {
    color: #c7d2fe;
    border-left-color: #818cf8;
    background: rgba(99, 102, 241, 0.12);
  }
</style>
