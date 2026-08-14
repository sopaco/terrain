import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { blobToBase64 } from "./clipboard";

/** Returns the chosen directory, or null when the user cancels. */
export async function pickDirectory(title: string): Promise<string | null> {
  const picked = await open({ directory: true, multiple: false, title });
  return typeof picked === "string" ? picked : null;
}

/** Writes each page as `<baseName>-01.png` …; returns the paths actually written. */
export async function savePngPages(
  dir: string,
  baseName: string,
  pages: Blob[],
): Promise<string[]> {
  const pngsBase64 = await Promise.all(pages.map(blobToBase64));
  return await invoke<string[]>("save_png_files", { dir, baseName, pngsBase64 });
}

/** `terrain-ask-<question head>-<yyyymmdd-hhmm>`; the backend sanitises it. */
export function shareFileBaseName(question: string, now = new Date()): string {
  const pad = (value: number) => String(value).padStart(2, "0");
  const stamp =
    `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}` +
    `-${pad(now.getHours())}${pad(now.getMinutes())}`;
  const head = question.replace(/\s+/g, "-").slice(0, 24);
  return `terrain-ask-${head}-${stamp}`;
}
