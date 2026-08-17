<script lang="ts">
  import MarkdownViewer from "./MarkdownViewer.svelte";
  import { tr } from "../i18n";
  import appIconUrl from "../../../assets/app-icon.png";

  /**
   * Off-screen render target for Ask share images. Deliberately renders the real
   * `MarkdownViewer` instead of a private markdown pipeline: the share image then
   * inherits `markdown.css`, syntax highlighting and rendered mermaid diagrams,
   * so it cannot drift from what the user sees on screen.
   *
   * The `data-share-*` hooks are driven imperatively by `askShareImage.ts` while
   * it paginates — one mount is reshot once per page.
   */
  interface Props {
    question: string;
    answerMarkdown: string;
    generatedAt: string;
    /** Off while the answer still streams: half-written diagrams cannot render. */
    allowMermaid?: boolean;
  }

  let { question, answerMarkdown, generatedAt, allowMermaid = true }: Props = $props();

  /** A wall of question text would crowd out the answer it introduces. */
  const QUESTION_LIMIT = 420;
  const RECAP_LIMIT = 56;

  function clamp(text: string, limit: number): string {
    return text.length > limit ? `${text.slice(0, limit)}…` : text;
  }

  const shownQuestion = $derived(clamp(question, QUESTION_LIMIT));
  const recap = $derived(clamp(question.replace(/\s+/g, " ").trim(), RECAP_LIMIT));
</script>

<div class="frame" data-share-frame>
  <div class="card">
    <header class="head">
      <div class="brand">
        <img class="mark" src={appIconUrl} alt="" />
        <span class="brand-name">Terrain</span>
      </div>
      <div class="meta">
        <span class="meta-date">{generatedAt}</span>
        <span class="page" data-share-page hidden><span class="page-num"></span></span>
      </div>
    </header>

    <div class="ask" data-share-question>
      <span class="ask-label">{tr("ask.share.questionLabel")}</span>
      <p class="ask-text">{shownQuestion}</p>
    </div>

    <p class="cont" data-share-continuation hidden>{tr("ask.share.continued", { recap })}</p>

    <div class="answer" data-share-answer>
      <MarkdownViewer body={answerMarkdown} compact {allowMermaid} highlight />
    </div>

    <p class="omitted" data-share-omitted hidden></p>

    <footer class="foot">{tr("ask.share.generatedBy")}</footer>
  </div>
</div>

<style>
  .frame {
    box-sizing: border-box;
    width: 100%;
    padding: 20px;
    background: var(--color-tr-page);
    font-family: Inter, ui-sans-serif, system-ui, -apple-system, sans-serif;
  }

  .frame [hidden] {
    display: none !important;
  }

  .card {
    box-sizing: border-box;
    padding: 22px 24px 18px;
    border: 1px solid var(--color-tr-border-strong);
    border-radius: var(--radius-2xl);
    background: var(--color-tr-surface);
    color: var(--color-tr-ink);
  }

  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 16px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--color-tr-border);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  /* The icon is already a filled rounded-square glyph — no extra box around it. */
  .mark {
    display: block;
    width: 26px;
    height: 26px;
    object-fit: contain;
  }

  /*
   * html2canvas draws each text node from its own estimated font baseline
   * rather than the browser's real one, so a glyph paints noticeably below
   * the center of its (correctly measured) box — verified against this exact
   * card at roughly 0.55–0.58× font-size, consistently, while an <img> in the
   * same row is unaffected. On screen this transform is invisible (no other
   * element to compare against); it only matters once rasterised next to the
   * icon, which is why the fix lives here and not in a shared stylesheet.
   */
  .brand-name,
  .meta-date,
  .page-num {
    display: inline-block;
    transform: translateY(-0.56em);
  }

  .brand-name {
    font-size: 14px;
    font-weight: 600;
    line-height: 1.3;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--color-tr-ink-3);
  }

  .page {
    padding: 1px 8px;
    border: 1px solid var(--color-tr-border-strong);
    border-radius: 999px;
    color: var(--color-tr-ink-2);
  }

  .ask {
    margin-bottom: 16px;
    padding: 12px 14px;
    border: 1px solid var(--color-tr-accent-soft-strong);
    border-radius: var(--radius-xl);
    background: var(--color-tr-accent-soft);
  }

  .ask-label {
    display: block;
    margin-bottom: 6px;
    font-size: 11px;
    font-weight: 600;
    color: var(--color-tr-accent);
  }

  .ask-text {
    margin: 0;
    font-size: 14px;
    line-height: 1.65;
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--color-tr-ink);
  }

  .cont {
    margin: 0 0 14px;
    padding: 8px 12px;
    border: 1px dashed var(--color-tr-border-strong);
    border-radius: var(--radius-xl);
    background: var(--color-tr-elevated);
    font-size: 12px;
    word-break: break-word;
    color: var(--color-tr-ink-3);
  }

  .omitted {
    margin: 14px 0 0;
    padding: 9px 12px;
    border: 1px solid rgba(217, 164, 65, 0.32);
    border-radius: var(--radius-xl);
    background: var(--color-tr-watch-soft);
    font-size: 12px;
    color: var(--color-tr-watch);
  }

  .foot {
    margin-top: 18px;
    padding-top: 12px;
    border-top: 1px solid var(--color-tr-border);
    font-size: 11px;
    text-align: center;
    color: var(--color-tr-ink-3);
  }

  /*
   * Rasterising clips overflow instead of scrolling it, so every "scrolls on
   * screen" box has to wrap in the share image or its content is silently cut.
   */
  .answer :global(.markdown-body pre),
  .answer :global(.markdown-body .code-block),
  .answer :global(.markdown-body .markdown-table-wrap),
  .answer :global(.markdown-body .mermaid-wrap) {
    overflow: visible;
  }

  .answer :global(.markdown-body pre code) {
    white-space: pre-wrap;
    word-break: break-word;
  }

  .answer :global(.markdown-body th),
  .answer :global(.markdown-body td) {
    word-break: break-word;
  }

  /* Hover-only affordances that would just be dead chrome in a static image. */
  .answer :global(.markdown-body .code-copy) {
    display: none;
  }

  /* Rasterised letter-spacing advances by glyph width alone, which squeezes
     tracked-out labels. Normal spacing renders truthfully. */
  .answer :global(.markdown-body .code-block-lang) {
    letter-spacing: normal;
  }

  .answer :global(.markdown-body .source-ref),
  .answer :global(.markdown-body .mermaid-wrap) {
    cursor: default;
  }

  .answer :global(.markdown-body .mermaid-wrap img) {
    display: block;
    margin: 0 auto;
    max-width: 100%;
  }

  /* Page breaks land between blocks; the block that opens a page owns no gap. */
  .answer :global(.markdown-body > [data-share-first]) {
    margin-top: 0 !important;
  }

  .answer :global(.markdown-body > :last-child) {
    margin-bottom: 0;
  }
</style>
