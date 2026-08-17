import type { LanguageSetting } from "../generated/LanguageSetting";

export type Locale = "zh-CN" | "en";

/**
 * Active UI locale. Defaults to Chinese; `initLocale` sets the real value at
 * bootstrap. Wrapped in an object so the export can be mutated without
 * reassignment (required by Svelte 5 module state rules).
 */
export const locale = $state<{ current: Locale }>({ current: "zh-CN" });

export function detectSystemLocale(): Locale {
  const nav =
    typeof navigator !== "undefined" ? navigator.language || "" : "";
  return nav.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

/** Resolve a persisted setting to a concrete locale. */
export function resolveLocale(
  setting: LanguageSetting | null | undefined,
): Locale {
  if (setting === "zh-CN" || setting === "en") return setting;
  return detectSystemLocale();
}

/** Call once at app bootstrap and whenever the user saves language settings. */
export function initLocale(setting: LanguageSetting | null | undefined): void {
  locale.current = resolveLocale(setting);
}
