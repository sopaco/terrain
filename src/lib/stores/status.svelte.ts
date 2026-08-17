import type { StatusKind } from "../components/StatusBanner.svelte";
import { t } from "../i18n";

export const STATUS_AUTO_DISMISS_MS = 3_000;

const DEFAULT_KIND: StatusKind = "idle";

function defaultMessage(): string {
  return t("terms.statusChip.idle");
}

export const status = $state<{
  message: string;
  kind: StatusKind;
  detail: string | null;
}>({
  message: defaultMessage(),
  kind: DEFAULT_KIND,
  detail: null,
});

let dismissTimer: ReturnType<typeof setTimeout> | undefined;
let dismissGeneration = 0;

export function clearStatus() {
  dismissGeneration++;
  if (dismissTimer) {
    clearTimeout(dismissTimer);
    dismissTimer = undefined;
  }
  status.message = defaultMessage();
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
  const dismissMs =
    autoDismissMs !== undefined
      ? autoDismissMs
      : kind === "success"
        ? STATUS_AUTO_DISMISS_MS
        : undefined;
  if (dismissMs != null && dismissMs > 0) {
    dismissTimer = setTimeout(() => {
      if (generation === dismissGeneration) {
        clearStatus();
      }
    }, dismissMs);
  }
}
