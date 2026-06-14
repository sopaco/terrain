export function escapeHtml(text: string): string {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

/** True when every fenced ```mermaid block in the markdown is closed. */
export function hasCompleteMermaidBlocks(markdown: string): boolean {
  const re = /```mermaid[^\n]*\n[\s\S]*?```/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = re.exec(markdown)) !== null) {
    lastIndex = match.index + match[0].length;
  }
  const tail = markdown.slice(lastIndex);
  return !tail.includes("```mermaid");
}

export function prepareSvgForExport(svg: string): string {
  const doc = new DOMParser().parseFromString(svg, "image/svg+xml");
  const el = doc.documentElement;
  if (doc.querySelector("parsererror")) {
    throw new Error("invalid SVG markup");
  }
  if (!el.getAttribute("width") || !el.getAttribute("height")) {
    const viewBox = el.getAttribute("viewBox")?.trim().split(/\s+/);
    if (viewBox?.length === 4) {
      el.setAttribute("width", viewBox[2]);
      el.setAttribute("height", viewBox[3]);
    } else {
      el.setAttribute("width", "1200");
      el.setAttribute("height", "800");
    }
  }
  if (!el.getAttribute("xmlns")) {
    el.setAttribute("xmlns", "http://www.w3.org/2000/svg");
  }
  return new XMLSerializer().serializeToString(el);
}

export async function svgToPngBlob(svg: string): Promise<Blob> {
  const prepared = prepareSvgForExport(svg);
  const svgBase64 = btoa(unescape(encodeURIComponent(prepared)));
  const dataUrl = `data:image/svg+xml;base64,${svgBase64}`;

  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => {
      const padding = 32;
      const canvas = document.createElement("canvas");
      canvas.width = Math.max(1, img.width + padding * 2);
      canvas.height = Math.max(1, img.height + padding * 2);
      const ctx = canvas.getContext("2d");
      if (!ctx) {
        reject(new Error("canvas unavailable"));
        return;
      }
      ctx.fillStyle = "#12151c";
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      ctx.drawImage(img, padding, padding);
      canvas.toBlob(
        (blob) => (blob ? resolve(blob) : reject(new Error("png export failed"))),
        "image/png",
      );
    };
    img.onerror = () => reject(new Error("svg load failed"));
    img.src = dataUrl;
  });
}
