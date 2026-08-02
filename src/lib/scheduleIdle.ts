/** Run `task` when the browser is idle, with a hard timeout fallback. */
export function scheduleIdle(task: () => void, timeoutMs = 8_000): void {
  if (typeof requestIdleCallback === "function") {
    requestIdleCallback(() => task(), { timeout: timeoutMs });
    return;
  }
  window.setTimeout(task, Math.min(timeoutMs, 3_000));
}
