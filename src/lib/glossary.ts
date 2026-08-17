/**
 * In-app user dictionary. Locale-aware: derived from the active i18n locale.
 * Use as `$derived(buildGlossary())` in components (see HelpPanel).
 */
import { tr } from "./i18n";

export type GlossaryEntry = {
  term: string;
  description: string;
};

const ENTRY_KEYS = [
  "knowledgeTab",
  "humanKnowledge",
  "agentKnowledge",
  "addAndInit",
  "rebuildIndex",
  "ask",
  "quickRefresh",
  "freshness",
  "agentEnv",
  "structuredIndex",
  "acp",
  "llm",
  "sdd",
] as const;

export function buildGlossary(): GlossaryEntry[] {
  return ENTRY_KEYS.map((key) => ({
    term: tr(`glossary.${key}.term`),
    description: tr(`glossary.${key}.description`),
  }));
}
