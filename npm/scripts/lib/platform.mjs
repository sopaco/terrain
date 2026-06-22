/** @returns {string} e.g. `darwin-arm64` */
export function platformKey() {
  return `${process.platform}-${process.arch}`;
}

/** @param {string} prefix e.g. `@terrain-ai/rtk` */
export function platformPackageName(prefix) {
  return `${prefix}-${platformKey()}`;
}
