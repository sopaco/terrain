/**
 * Lightweight i18n for the Terrain GUI.
 *
 * - The persisted preference is `language` in `~/.terrain/settings.json`
 *   (`LanguageSetting`: "system" | "zh-CN" | "en"), shared with the CLI and
 *   the Rust backend. "system" resolves via `navigator.language`.
 * - Dictionaries live in `./locales/<locale>/<module>.ts`; `zh-CN` is the
 *   schema source, `en` must mirror the same keys.
 * - Usage in components: `import { tr } from "$lib/i18n";` then
 *   `{tr("settings.title")}` or `{tr("freshness.behind", { count: 4 })}`.
 *   `tr` reads reactive `$state` locale, so templates and `$derived` update
 *   immediately when language changes.
 */
import type { LanguageSetting } from "../generated/LanguageSetting";
import { zhCN } from "./locales/zh-CN";
import { en } from "./locales/en";
import {
  detectSystemLocale,
  initLocale,
  locale,
  resolveLocale,
  type Locale,
} from "./locale.svelte";

export type { Locale, LanguageSetting };
export { detectSystemLocale, initLocale, locale, resolveLocale };

export type Messages = typeof zhCN;

const DICTS: Record<Locale, Messages> = {
  "zh-CN": zhCN,
  // Cast: `en` mirrors the zh-CN schema; any key missing during incremental
  // roll-out falls back to zh-CN at runtime (see `translate`).
  en: en as unknown as Messages,
};

function lookup(
  dict: unknown,
  path: string,
): string | undefined {
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
  const value =
    lookup(DICTS[loc], key) ?? lookup(DICTS["zh-CN"], key);
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
