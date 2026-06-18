import type { StatusKind } from "../components/StatusBanner.svelte";

export const status = $state({
  message: "就绪",
  kind: "idle" as StatusKind,
  detail: null as string | null,
});

export function setStatus(
  message: string,
  kind: StatusKind = "idle",
  detail: string | null = null,
) {
  status.message = message;
  status.kind = kind;
  status.detail = detail;
}
