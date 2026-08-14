import { invoke } from "@tauri-apps/api/core";
import { prepareSvgForExport, svgToPngBlob } from "./mermaid-utils";

export async function copyTextToClipboard(text: string): Promise<void> {
  await invoke("copy_text_to_clipboard", { text });
}

/** Chunked rather than per-byte: a share image runs to several megabytes. */
export async function blobToBase64(blob: Blob): Promise<string> {
  const bytes = new Uint8Array(await blob.arrayBuffer());
  const chunkSize = 0x8000;
  let binary = "";
  for (let i = 0; i < bytes.length; i += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(i, i + chunkSize));
  }
  return btoa(binary);
}

export async function copyPngBlobToClipboard(blob: Blob): Promise<void> {
  await invoke("copy_image_to_clipboard", { pngBase64: await blobToBase64(blob) });
}

export async function copySvgAsImage(svg: string): Promise<"image" | "text"> {
  try {
    const blob = await svgToPngBlob(svg);
    await copyPngBlobToClipboard(blob);
    return "image";
  } catch {
    await invoke("copy_text_to_clipboard", { text: prepareSvgForExport(svg) });
    return "text";
  }
}
