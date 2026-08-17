import { zhCN } from "./locales/zh-CN";
import { en } from "./locales/en";
import { locale, type Locale } from "./locale.svelte";

export type Messages = typeof zhCN;

const DICTS: Record<Locale, Messages> = {
  "zh-CN": zhCN,
  en: en as unknown as Messages,
};

function lookup(dict: unknown, path: string): string | undefined {
  let node = dict;
  for (const part of path.split(".")) {
    if (node == null || typeof node !== "object") return undefined;
    node = (node as Record<string, unknown>)[part];
  }
  return typeof node === "string" ? node : undefined;
}

function interpolate(
  template: string,
  params?: Record<string, string | number>,
): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match,
  );
}

export function translate(
  loc: Locale,
  key: string,
  params?: Record<string, string | number>,
): string {
  const value = lookup(DICTS[loc], key) ?? lookup(DICTS["zh-CN"], key);
  if (value === undefined) {
    console.warn(`[i18n] missing key: ${key}`);
    return key;
  }
  return interpolate(value, params);
}

/** Reactive translator for Svelte components and `$derived` blocks. */
export function tr(
  key: string,
  params?: Record<string, string | number>,
): string {
  return translate(locale.current, key, params);
}

/** Non-reactive imperative access for .ts modules outside components. */
export function t(
  key: string,
  params?: Record<string, string | number>,
): string {
  return translate(locale.current, key, params);
}

/** True when `message` is the idle chip label in any supported locale. */
export function isIdleStatusMessage(message: string): boolean {
  return (
    message === translate("zh-CN", "terms.statusChip.idle") ||
    message === translate("en", "terms.statusChip.idle")
  );
}
