/** Flatten nested locale objects into dotted keys (for parity checks). */
export function collectMessageKeys(
  node: unknown,
  prefix = "",
): string[] {
  if (node == null || typeof node !== "object") return [];
  const out: string[] = [];
  for (const [key, value] of Object.entries(node as Record<string, unknown>)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (typeof value === "string") {
      out.push(path);
    } else {
      out.push(...collectMessageKeys(value, path));
    }
  }
  return out.sort();
}
