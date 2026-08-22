import { mount, tick, unmount } from "svelte";
import AskShareCard from "./components/AskShareCard.svelte";
import { t } from "./i18n";
import { hasCompleteMermaidBlocks, prepareSvgForExport } from "./mermaid-utils";
import { formatShareStamp } from "./timeFormat";
import { formatErrorSummary } from "./errorFormat";

/** PNG width in CSS px, echoing the Ask panel so shared images feel like the app. */
const FRAME_WIDTH = 800;

/**
 * Target height of one page. A single answer can run tens of thousands of pixels;
 * one image that tall is unusable in a chat client (previews collapse it to a
 * line) and can exceed the WebKit canvas budget outright, so long answers are
 * split into several pages instead of stretched into one strip.
 */
const PAGE_TARGET_HEIGHT = 2400;
const MIN_PAGE_BUDGET = 600;
const MAX_PAGES = 12;

/**
 * WebKit refuses to back a canvas past roughly 16.7M pixels and silently hands
 * back a blank one, so the scale is chosen to stay under budget rather than
 * assuming 2x always fits.
 */
const CANVAS_AREA_BUDGET = 14_000_000;
const MAX_CANVAS_EDGE = 16_384;
const MAX_SCALE = 2;

const HIGHLIGHT_TIMEOUT_MS = 3_000;
const MERMAID_TIMEOUT_MS = 6_000;
const IMAGE_LOAD_TIMEOUT_MS = 2_000;
const FRAME_WAIT_FALLBACK_MS = 120;

export interface AskShareCardInput {
  question: string;
  answerMarkdown: string;
}

export interface AskShareImages {
  /** One PNG per page, in reading order. Never empty. */
  pages: Blob[];
  /** Top-level answer blocks dropped because the answer exceeded `MAX_PAGES`. */
  omittedBlocks: number;
}

export function formatUnknownError(error: unknown): string {
  if (error instanceof Event) return t("ask.shareImage.renderRetry");
  const summary = formatErrorSummary(error);
  return summary || t("ask.shareImage.unknownError");
}

interface Block {
  el: HTMLElement;
  top: number;
  bottom: number;
}

interface PageSlice {
  start: number;
  end: number;
}

interface PageBudgets {
  first: number;
  rest: number;
}

interface CardParts {
  frame: HTMLElement;
  question: HTMLElement;
  continuation: HTMLElement;
  /** The badge's border/background box — only its `hidden` state is toggled. */
  pageBadge: HTMLElement;
  /** The text run inside the badge — carries the html2canvas baseline fix, so
   *  page numbers are written here rather than on `pageBadge` directly. */
  pageBadgeText: HTMLElement;
  omitted: HTMLElement;
  body: HTMLElement;
}

/** Render an Ask Q&A pair into share-ready PNG pages. */
export async function renderAskShareImages(input: AskShareCardInput): Promise<AskShareImages> {
  const question = input.question.trim();
  const answerMarkdown = input.answerMarkdown.trim();
  if (!question || !answerMarkdown) {
    throw new Error("question and answer are required");
  }

  // Off-screen but genuinely laid out: html2canvas reads real client rects, and a
  // `display: none` or zero-width host would give it nothing to measure. Height 0
  // keeps a tall card from disturbing the app's own scroll height.
  const host = document.createElement("div");
  host.setAttribute("aria-hidden", "true");
  host.style.cssText =
    `position:absolute;left:-99999px;top:0;width:${FRAME_WIDTH}px;` +
    "height:0;overflow:visible;pointer-events:none;";
  document.body.appendChild(host);

  const card = mount(AskShareCard, {
    target: host,
    props: {
      question,
      answerMarkdown,
      generatedAt: formatShareStamp(Date.now()),
      allowMermaid: hasCompleteMermaidBlocks(answerMarkdown),
    },
  });

  try {
    return await shootPages(host);
  } finally {
    void unmount(card);
    host.remove();
  }
}

async function shootPages(host: HTMLElement): Promise<AskShareImages> {
  const parts = cardParts(host);
  await settleCard(parts.body);

  const blocks = measureBlocks(parts.body);
  const budgets = measureBudgets(parts);
  const slices = paginate(blocks, budgets);
  const omittedBlocks = blocks.length - (slices[slices.length - 1]?.end ?? blocks.length);

  const pages: Blob[] = [];
  for (const [index, slice] of slices.entries()) {
    applyPage(parts, blocks, slice, index, slices.length, omittedBlocks);
    await nextFrame();
    pages.push(await rasterize(parts.frame, host));
  }
  return { pages, omittedBlocks };
}

function cardParts(host: HTMLElement): CardParts {
  return {
    frame: requireEl(host, "[data-share-frame]"),
    question: requireEl(host, "[data-share-question]"),
    continuation: requireEl(host, "[data-share-continuation]"),
    pageBadge: requireEl(host, "[data-share-page]"),
    pageBadgeText: requireEl(host, "[data-share-page] .page-num"),
    omitted: requireEl(host, "[data-share-omitted]"),
    body: requireEl(host, "[data-share-answer] .markdown-body"),
  };
}

function requireEl(host: HTMLElement, selector: string): HTMLElement {
  const el = host.querySelector<HTMLElement>(selector);
  if (!el) throw new Error(`share card is missing ${selector}`);
  return el;
}

/**
 * Wait until the card is done becoming itself: fonts loaded, code highlighted,
 * diagrams drawn. Shooting early is how a share image ends up with unstyled code
 * or an empty diagram box. Each wait has a ceiling — a stalled dynamic import
 * should degrade the image, not hang the button.
 */
async function settleCard(body: HTMLElement): Promise<void> {
  await tick();
  await nextFrame();
  if (document.fonts?.ready) await document.fonts.ready;
  await waitUntil(() => allHighlighted(body), HIGHLIGHT_TIMEOUT_MS);
  await waitUntil(() => allMermaidSettled(body), MERMAID_TIMEOUT_MS);
  await inlineMermaidDiagrams(body);
  await nextFrame();
}

function allHighlighted(body: HTMLElement): boolean {
  return Array.from(body.querySelectorAll<HTMLElement>(".code-block code")).every(
    (el) => el.dataset.highlighted === "true",
  );
}

function allMermaidSettled(body: HTMLElement): boolean {
  return Array.from(body.querySelectorAll<HTMLElement>(".mermaid-wrap")).every(
    (el) => el.dataset.rendered === "true" || el.dataset.rendered === "error",
  );
}

/**
 * Swap each rendered diagram for a data-URI `<img>` of the same size. html2canvas
 * can draw a serialized SVG far more reliably than a live one, and inlining here
 * keeps the diagrams in the picture instead of dropping them for a placeholder.
 */
async function inlineMermaidDiagrams(body: HTMLElement): Promise<void> {
  for (const svg of Array.from(body.querySelectorAll<SVGSVGElement>(".mermaid-wrap svg"))) {
    const rect = svg.getBoundingClientRect();
    const width = Math.max(1, Math.round(rect.width));
    const height = Math.max(1, Math.round(rect.height));

    const clone = svg.cloneNode(true) as SVGSVGElement;
    clone.removeAttribute("style");
    clone.setAttribute("width", String(width));
    clone.setAttribute("height", String(height));

    let markup: string;
    try {
      markup = prepareSvgForExport(new XMLSerializer().serializeToString(clone));
    } catch {
      continue;
    }

    const img = document.createElement("img");
    img.src = `data:image/svg+xml;base64,${btoa(unescape(encodeURIComponent(markup)))}`;
    img.style.width = `${width}px`;
    img.style.height = `${height}px`;
    img.alt = "";
    // `decode()` can hang indefinitely on a detached SVG image, so wait on the
    // load events instead. A diagram that will not load stays a live <svg>.
    if (await imageLoaded(img, IMAGE_LOAD_TIMEOUT_MS)) {
      svg.replaceWith(img);
    }
  }
}

function measureBlocks(body: HTMLElement): Block[] {
  const bodyTop = body.getBoundingClientRect().top;
  return Array.from(body.children)
    .filter((el): el is HTMLElement => el instanceof HTMLElement)
    .map((el) => {
      const rect = el.getBoundingClientRect();
      return { el, top: rect.top - bodyTop, bottom: rect.bottom - bodyTop };
    });
}

/**
 * How much answer fits on a page, measured rather than guessed: page 1 carries the
 * question block, later pages carry the narrower "continued" line instead.
 */
function measureBudgets(parts: CardParts): PageBudgets {
  const chrome = () => Math.max(0, parts.frame.offsetHeight - parts.body.offsetHeight);
  const badgeWasHidden = parts.pageBadge.hidden;

  parts.pageBadge.hidden = false;
  parts.pageBadgeText.textContent = "1 / 9";
  parts.question.hidden = false;
  parts.continuation.hidden = true;
  const first = PAGE_TARGET_HEIGHT - chrome();

  parts.question.hidden = true;
  parts.continuation.hidden = false;
  const rest = PAGE_TARGET_HEIGHT - chrome();

  parts.question.hidden = false;
  parts.continuation.hidden = true;
  parts.pageBadge.hidden = badgeWasHidden;

  return {
    first: Math.max(MIN_PAGE_BUDGET, first),
    rest: Math.max(MIN_PAGE_BUDGET, rest),
  };
}

/**
 * Greedy block packing. A block never straddles a page boundary — cutting mid
 * paragraph or mid code block is exactly the kind of mangled output this replaces
 * — so a block taller than the budget gets a page to itself and is allowed to
 * overflow it.
 */
function paginate(blocks: Block[], budgets: PageBudgets): PageSlice[] {
  if (blocks.length === 0) return [{ start: 0, end: 0 }];

  const slices: PageSlice[] = [];
  let start = 0;
  for (let i = 0; i < blocks.length; i += 1) {
    const budget = slices.length === 0 ? budgets.first : budgets.rest;
    if (i > start && blocks[i].bottom - blocks[start].top > budget) {
      slices.push({ start, end: i });
      start = i;
      if (slices.length >= MAX_PAGES) return slices;
    }
  }
  slices.push({ start, end: blocks.length });
  return slices;
}

function applyPage(
  parts: CardParts,
  blocks: Block[],
  slice: PageSlice,
  index: number,
  total: number,
  omittedBlocks: number,
): void {
  blocks.forEach((block, i) => {
    const visible = i >= slice.start && i < slice.end;
    block.el.style.display = visible ? "" : "none";
    if (visible && i === slice.start) {
      block.el.setAttribute("data-share-first", "");
    } else {
      block.el.removeAttribute("data-share-first");
    }
  });

  parts.question.hidden = index !== 0;
  parts.continuation.hidden = index === 0;
  parts.pageBadge.hidden = total < 2;
  parts.pageBadgeText.textContent = `${index + 1} / ${total}`;

  const showOmitted = index === total - 1 && omittedBlocks > 0;
  parts.omitted.hidden = !showOmitted;
  parts.omitted.textContent = showOmitted
    ? t("ask.shareImage.omitted", { count: omittedBlocks })
    : "";
}

async function rasterize(frame: HTMLElement, host: HTMLElement): Promise<Blob> {
  const { default: html2canvas } = await import("html2canvas");
  const width = frame.offsetWidth;
  const height = frame.offsetHeight;
  let lastError: unknown = null;

  for (const scale of scaleCandidates(width, height)) {
    try {
      const canvas = await html2canvas(frame, {
        backgroundColor: pageBackground(),
        scale,
        useCORS: true,
        logging: false,
        // The clone would otherwise duplicate and re-lay-out the whole app for
        // every page. `<head>` is left alone so the app's real CSS still applies.
        ignoreElements: (el) => el.parentElement === document.body && el !== host,
      });
      if (canvasLooksPainted(canvas)) return await canvasToPngBlob(canvas);
      lastError = new Error(t("ask.shareImage.canvasTooLarge", { width, height }));
    } catch (error) {
      lastError = error;
    }
  }
  throw new Error(formatUnknownError(lastError ?? new Error(t("ask.shareImage.renderFailed"))));
}

/** Highest scale that fits the canvas budget, then progressively safer fallbacks. */
function scaleCandidates(width: number, height: number): number[] {
  const area = Math.max(1, width * height);
  const edgeLimit = Math.min(
    MAX_CANVAS_EDGE / Math.max(1, width),
    MAX_CANVAS_EDGE / Math.max(1, height),
  );
  const best = Math.min(MAX_SCALE, Math.sqrt(CANVAS_AREA_BUDGET / area), edgeLimit);
  const rounded = [best, best * 0.75, 1, 0.6]
    .filter((scale) => scale <= edgeLimit)
    .map((scale) => Math.max(0.5, Math.round(scale * 100) / 100));
  return Array.from(new Set(rounded));
}

/**
 * An oversized canvas comes back fully transparent instead of throwing, so sample
 * a few pixels: the card paints an opaque background everywhere it succeeded.
 */
function canvasLooksPainted(canvas: HTMLCanvasElement): boolean {
  if (canvas.width === 0 || canvas.height === 0) return false;
  const ctx = canvas.getContext("2d");
  if (!ctx) return false;

  const maxX = canvas.width - 1;
  const maxY = canvas.height - 1;
  const points: [number, number][] = [
    [Math.min(1, maxX), Math.min(1, maxY)],
    [maxX >> 1, maxY >> 1],
    [Math.max(0, maxX - 1), Math.max(0, maxY - 1)],
  ];
  try {
    return points.some(([x, y]) => ctx.getImageData(x, y, 1, 1).data[3] > 0);
  } catch {
    // Tainted canvas — unreadable, but that means it drew something.
    return true;
  }
}

function pageBackground(): string {
  const value = getComputedStyle(document.documentElement)
    .getPropertyValue("--color-tr-page")
    .trim();
  return value || "#0a0d10";
}

async function canvasToPngBlob(canvas: HTMLCanvasElement): Promise<Blob> {
  const blob = await new Promise<Blob | null>((resolve) => {
    canvas.toBlob((value) => resolve(value), "image/png");
  });
  if (!blob?.size) throw new Error("png export failed");
  return blob;
}

/**
 * Two frames, or a timer — whichever lands first. A hidden or minimised window
 * stops servicing `requestAnimationFrame` entirely, and waiting on a frame that
 * will never arrive would leave the share button spinning forever.
 */
function nextFrame(): Promise<void> {
  return new Promise<void>((resolve) => {
    let settled = false;
    const settle = () => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve();
    };
    const timer = setTimeout(settle, FRAME_WAIT_FALLBACK_MS);
    requestAnimationFrame(() => requestAnimationFrame(settle));
  });
}

function imageLoaded(img: HTMLImageElement, timeoutMs: number): Promise<boolean> {
  const decoded = () => img.complete && img.naturalWidth > 0;
  if (decoded()) return Promise.resolve(true);
  return new Promise<boolean>((resolve) => {
    const settle = (ok: boolean) => {
      clearTimeout(timer);
      img.onload = null;
      img.onerror = null;
      resolve(ok);
    };
    const timer = setTimeout(() => settle(decoded()), timeoutMs);
    img.onload = () => settle(true);
    img.onerror = () => settle(false);
  });
}

async function waitUntil(check: () => boolean, timeoutMs: number): Promise<void> {
  const deadline = performance.now() + timeoutMs;
  while (!check()) {
    if (performance.now() > deadline) return;
    await new Promise<void>((resolve) => {
      setTimeout(resolve, 40);
    });
  }
}
