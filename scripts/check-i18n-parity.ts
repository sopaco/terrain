/**
 * Verify zh-CN and en locale dictionaries expose the same dotted keys.
 * Run: bun run check:i18n
 */
import { collectMessageKeys } from "../src/lib/i18n/keys.ts";
import { zhCN } from "../src/lib/i18n/locales/zh-CN/index.ts";
import { en } from "../src/lib/i18n/locales/en/index.ts";

const zh = collectMessageKeys(zhCN);
const enKeys = collectMessageKeys(en);
const zhSet = new Set(zh);
const enSet = new Set(enKeys);

const missingInEn = zh.filter((key) => !enSet.has(key));
const missingInZh = enKeys.filter((key) => !zhSet.has(key));

if (missingInEn.length > 0 || missingInZh.length > 0) {
  console.error("i18n key parity check failed.");
  if (missingInEn.length > 0) {
    console.error(`Missing in en (${missingInEn.length}):`);
    for (const key of missingInEn) console.error(`  - ${key}`);
  }
  if (missingInZh.length > 0) {
    console.error(`Missing in zh-CN (${missingInZh.length}):`);
    for (const key of missingInZh) console.error(`  - ${key}`);
  }
  process.exit(1);
}

console.log(`i18n parity OK (${zh.length} keys).`);
