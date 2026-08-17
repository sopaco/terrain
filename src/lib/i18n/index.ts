/**
 * Lightweight i18n for the Terrain GUI.
 *
 * - The persisted preference is `language` in `~/.terrain/settings.json`
 *   (`LanguageSetting`: "system" | "zh-CN" | "en`), shared with the CLI and
 *   the Rust backend. "system" resolves via `navigator.language`.
 * - Dictionaries live in `./locales/<locale>/<module>.ts`; `zh-CN` is the
 *   schema source, `en` must mirror the same keys.
 * - Usage in components: `import { tr } from "$lib/i18n";` then
 *   `{tr("settings.title")}` or `{tr("freshness.behind", { count: 4 })}`.
 *
 * Rust UI strings use `language::ResolvedLanguage::tr(zh, en)`; keep wording
 * aligned with matching frontend keys where possible (see `scripts/check-i18n-parity.ts`).
 */
import type { LanguageSetting } from "../generated/LanguageSetting";
import { getModelSettings } from "../api";
import { syncStatusLocale } from "../stores/status.svelte";
import {
  detectSystemLocale,
  initLocale,
  locale,
  resolveLocale,
  type Locale,
} from "./locale.svelte";
import {
  isIdleStatusMessage,
  t,
  tr,
  translate,
  type Messages,
} from "./translate";
import { collectMessageKeys } from "./keys";

export type { Locale, LanguageSetting, Messages };
export {
  collectMessageKeys,
  detectSystemLocale,
  initLocale,
  isIdleStatusMessage,
  locale,
  resolveLocale,
  t,
  tr,
  translate,
};

/** Load persisted language before first paint (main + usage windows). */
export async function bootstrapLocale(): Promise<void> {
  try {
    const settings = await getModelSettings();
    applyLocale(settings.language);
  } catch {
    applyLocale("system");
  }
}

/**
 * Set locale from persisted settings and refresh dependent UI (status bar, etc.).
 * Call at app bootstrap and whenever the user saves language settings.
 */
export function applyLocale(setting: LanguageSetting | null | undefined): void {
  initLocale(setting);
  syncStatusLocale();
}
