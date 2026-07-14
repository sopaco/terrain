import html2canvas from "html2canvas";
import { marked } from "marked";
import { escapeHtml } from "./mermaid-utils";
import { prepareMarkdownForRender } from "./markdownSanitize";

const CARD_WIDTH = 720;

const COLORS = {
  page: "#0a0d10",
  surface: "#12161b",
  elevated: "#181d23",
  border: "rgba(255, 255, 255, 0.08)",
  borderStrong: "rgba(255, 255, 255, 0.14)",
  ink: "#e8ecef",
  ink2: "#a2acb5",
  ink3: "#6c7680",
  accent: "#1f8f84",
  accentSoft: "rgba(31, 143, 132, 0.16)",
  onAccent: "#f3fbfa",
} as const;

const FONT =
  'Inter, ui-sans-serif, system-ui, -apple-system, "PingFang SC", "Microsoft YaHei", sans-serif';
const MONO = 'ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace';

export interface AskShareCardInput {
  question: string;
  answerMarkdown: string;
  projectName?: string | null;
}

export function formatUnknownError(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (error instanceof Event) return "图片渲染失败，请稍后重试";
  if (typeof error === "string") return error;
  return "未知错误";
}

function markdownToShareHtml(markdown: string): string {
  const body = prepareMarkdownForRender(markdown);
  const renderer = new marked.Renderer();
  renderer.code = ({ text, lang }) => {
    if (lang === "mermaid") {
      return `<div style="margin:0 0 12px;padding:10px 12px;border-radius:8px;border:1px dashed ${COLORS.borderStrong};background:${COLORS.elevated};color:${COLORS.ink3};font-size:12px;">图表（分享图片中省略）</div>`;
    }
    const language = lang ? ` class="language-${escapeHtml(lang)}"` : "";
    return `<pre style="margin:0 0 12px;padding:12px 14px;border-radius:10px;border:1px solid ${COLORS.border};background:${COLORS.page};overflow-x:auto;"><code${language} style="font-family:${MONO};font-size:12px;line-height:1.65;color:${COLORS.ink};white-space:pre-wrap;word-break:break-word;">${escapeHtml(text)}</code></pre>`;
  };
  renderer.codespan = ({ text }) =>
    `<code style="font-family:${MONO};font-size:0.9em;padding:0.1em 0.35em;border-radius:3px;background:${COLORS.elevated};color:${COLORS.accent};">${escapeHtml(text)}</code>`;
  renderer.link = ({ text }) =>
    `<span style="color:${COLORS.accent};text-decoration:underline;">${text}</span>`;
  renderer.image = () =>
    `<div style="margin:0 0 12px;padding:10px 12px;border-radius:8px;border:1px dashed ${COLORS.borderStrong};background:${COLORS.elevated};color:${COLORS.ink3};font-size:12px;">图片（分享图片中省略）</div>`;

  const html = marked.parse(body, { renderer, async: false }) as string;
  return `<div class="ask-share-md">${html}</div>`;
}

function shareMarkdownStyleText(): string {
  return `
    .ask-share-md { font-size:14px;line-height:1.7;color:${COLORS.ink};word-wrap:break-word; }
    .ask-share-md h1,.ask-share-md h2,.ask-share-md h3,.ask-share-md h4,.ask-share-md h5,.ask-share-md h6 { font-weight:600;line-height:1.35;color:${COLORS.ink};margin:0 0 10px; }
    .ask-share-md h1 { font-size:22px; }
    .ask-share-md h2 { font-size:18px;padding-bottom:4px;border-bottom:1px solid ${COLORS.border}; }
    .ask-share-md h3 { font-size:16px; }
    .ask-share-md h4 { font-size:15px; }
    .ask-share-md p,.ask-share-md ul,.ask-share-md ol,.ask-share-md blockquote,.ask-share-md table,.ask-share-md hr { margin:0 0 12px; }
    .ask-share-md ul,.ask-share-md ol { padding-left:1.4em; }
    .ask-share-md li { margin:0 0 4px; }
    .ask-share-md blockquote { padding:8px 12px;border-left:3px solid ${COLORS.accent};background:${COLORS.elevated};border-radius:0 8px 8px 0;color:${COLORS.ink2}; }
    .ask-share-md table { width:100%;border-collapse:collapse;font-size:13px; }
    .ask-share-md th,.ask-share-md td { padding:8px 10px;border:1px solid ${COLORS.borderStrong};text-align:left; }
    .ask-share-md th { background:${COLORS.elevated};font-weight:600; }
    .ask-share-md hr { border:none;border-top:1px solid ${COLORS.borderStrong};margin:16px 0; }
    .ask-share-md strong { font-weight:600;color:${COLORS.ink}; }
  `;
}

const SHARE_MD_STYLE_ID = "terrain-ask-share-md";

function mountShareMarkdownStyles(): void {
  if (document.getElementById(SHARE_MD_STYLE_ID)) return;
  const style = document.createElement("style");
  style.id = SHARE_MD_STYLE_ID;
  style.textContent = shareMarkdownStyleText();
  document.head.appendChild(style);
}

function unmountShareMarkdownStyles(): void {
  document.getElementById(SHARE_MD_STYLE_ID)?.remove();
}

function buildShareCardHtml(input: AskShareCardInput): string {
  const question = escapeHtml(input.question.trim());
  const answerHtml = markdownToShareHtml(input.answerMarkdown);
  const project = input.projectName?.trim();
  const headerSubtitle = project
    ? `<span style="color:${COLORS.ink3};font-size:12px;">${escapeHtml(project)}</span>`
    : `<span style="color:${COLORS.ink3};font-size:12px;">Ask</span>`;

  return `
    <div style="box-sizing:border-box;width:${CARD_WIDTH}px;padding:28px;font-family:${FONT};background:${COLORS.page};color:${COLORS.ink};">
      <div style="display:flex;align-items:center;justify-content:space-between;gap:12px;margin-bottom:20px;padding-bottom:16px;border-bottom:1px solid ${COLORS.borderStrong};">
        <div style="display:flex;align-items:center;gap:10px;">
          <div style="width:32px;height:32px;border-radius:10px;background:linear-gradient(135deg, ${COLORS.accent}, ${COLORS.page});display:flex;align-items:center;justify-content:center;color:${COLORS.onAccent};font-size:14px;font-weight:700;">T</div>
          <div style="display:flex;flex-direction:column;gap:2px;">
            <span style="font-size:14px;font-weight:600;color:${COLORS.ink};">Terrain</span>
            ${headerSubtitle}
          </div>
        </div>
      </div>
      <div style="margin-bottom:16px;padding:14px 16px;border-radius:12px;background:${COLORS.accentSoft};border:1px solid rgba(31, 143, 132, 0.28);">
        <div style="margin-bottom:8px;font-size:10px;font-weight:600;letter-spacing:0.08em;text-transform:uppercase;color:${COLORS.accent};">You</div>
        <div style="font-size:14px;line-height:1.65;color:${COLORS.ink};white-space:pre-wrap;word-break:break-word;">${question}</div>
      </div>
      <div style="padding:14px 16px;border-radius:12px;background:${COLORS.surface};border:1px solid ${COLORS.borderStrong};">
        <div style="margin-bottom:10px;font-size:10px;font-weight:600;letter-spacing:0.08em;text-transform:uppercase;color:${COLORS.ink3};">Terrain</div>
        ${answerHtml}
      </div>
      <div style="margin-top:18px;padding-top:14px;border-top:1px solid ${COLORS.border};font-size:11px;color:${COLORS.ink3};text-align:center;">
        Generated by Terrain
      </div>
    </div>
  `;
}

async function canvasToPngBlob(canvas: HTMLCanvasElement): Promise<Blob> {
  const blob = await new Promise<Blob | null>((resolve) => {
    canvas.toBlob((value) => resolve(value), "image/png");
  });
  if (!blob?.size) throw new Error("png export failed");
  return blob;
}

async function htmlToPngBlob(html: string, width: number): Promise<Blob> {
  const host = document.createElement("div");
  host.style.cssText =
    "position:fixed;left:-12000px;top:0;width:0;height:0;overflow:visible;pointer-events:none;";
  host.innerHTML = html.trim();
  const card = host.firstElementChild;
  if (!(card instanceof HTMLElement)) {
    throw new Error("share card markup missing");
  }
  document.body.appendChild(host);
  mountShareMarkdownStyles();

  try {
    await document.fonts.ready;
    await new Promise<void>((resolve) => {
      requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
    });

    const height = Math.max(1, card.scrollHeight);
    const canvas = await html2canvas(card, {
      backgroundColor: COLORS.page,
      scale: 2,
      width,
      height,
      windowWidth: width,
      windowHeight: height,
      useCORS: true,
      logging: false,
      onclone: (doc) => {
        const style = doc.createElement("style");
        style.textContent = shareMarkdownStyleText();
        doc.head.appendChild(style);
      },
    });
    return canvasToPngBlob(canvas);
  } catch (error) {
    throw new Error(formatUnknownError(error));
  } finally {
    document.body.removeChild(host);
    unmountShareMarkdownStyles();
  }
}

/** Render an Ask Q&A pair into a PNG blob suitable for sharing. */
export async function renderAskSharePng(input: AskShareCardInput): Promise<Blob> {
  const question = input.question.trim();
  const answer = input.answerMarkdown.trim();
  if (!question || !answer) {
    throw new Error("question and answer are required");
  }
  const html = buildShareCardHtml({ ...input, question, answerMarkdown: answer });
  return htmlToPngBlob(html, CARD_WIDTH);
}
