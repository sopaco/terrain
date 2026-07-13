import type { StatusKind } from "../components/StatusBanner.svelte";

export const STATUS_AUTO_DISMISS_MS = 3_000;

const DEFAULT_MESSAGE = "就绪";
const DEFAULT_KIND: StatusKind = "idle";

export const status = $state({
  message: DEFAULT_MESSAGE,
  kind: DEFAULT_KIND,
  detail: null as string | null,
});

let dismissTimer: ReturnType<typeof setTimeout> | undefined;
let dismissGeneration = 0;

export function clearStatus() {
  dismissGeneration++;
  if (dismissTimer) {
    clearTimeout(dismissTimer);
    dismissTimer = undefined;
  }
  status.message = DEFAULT_MESSAGE;
  status.kind = DEFAULT_KIND;
  status.detail = null;
}

export function setStatus(
  message: string,
  kind: StatusKind = "idle",
  detail: string | null = null,
  autoDismissMs?: number,
) {
  dismissGeneration++;
  const generation = dismissGeneration;
  if (dismissTimer) {
    clearTimeout(dismissTimer);
    dismissTimer = undefined;
  }
  status.message = message;
  status.kind = kind;
  status.detail = detail;
  if (autoDismissMs != null && autoDismissMs > 0) {
    dismissTimer = setTimeout(() => {
      if (generation === dismissGeneration) {
        clearStatus();
      }
    }, autoDismissMs);
  }
}
