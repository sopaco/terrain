/** True while an IME (e.g. Chinese Pinyin) composition is in progress. */
export function isImeComposing(e: KeyboardEvent): boolean {
  return e.isComposing || e.keyCode === 229;
}

/** Enter should submit only when not composing with IME. */
export function shouldSubmitOnEnter(e: KeyboardEvent): boolean {
  return e.key === "Enter" && !e.shiftKey && !isImeComposing(e);
}
