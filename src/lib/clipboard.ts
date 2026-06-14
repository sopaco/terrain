import { invoke } from "@tauri-apps/api/core";
import { prepareSvgForExport, svgToPngBlob } from "./mermaid-utils";

export async function copySvgAsImage(svg: string): Promise<"image" | "text"> {
  try {
    const blob = await svgToPngBlob(svg);
    const bytes = new Uint8Array(await blob.arrayBuffer());
    let binary = "";
    for (const byte of bytes) {
      binary += String.fromCharCode(byte);
    }
    const pngBase64 = btoa(binary);
    await invoke("copy_image_to_clipboard", { pngBase64 });
    return "image";
  } catch {
    await invoke("copy_text_to_clipboard", { text: prepareSvgForExport(svg) });
    return "text";
  }
}
