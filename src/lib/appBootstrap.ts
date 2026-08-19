import { bootstrapApp } from "./api";
import type { AppBootstrap } from "./types";

let cached: AppBootstrap | null = null;
let inflight: Promise<AppBootstrap> | null = null;

export function invalidateAppBootstrap(): void {
  cached = null;
  inflight = null;
}

/** Load app bootstrap payload; reuses cache unless `force` is set. */
export async function loadAppBootstrap(options?: {
  force?: boolean;
}): Promise<AppBootstrap> {
  if (options?.force) {
    invalidateAppBootstrap();
  }
  if (cached) return cached;
  if (inflight) return inflight;

  inflight = bootstrapApp()
    .then((boot) => {
      cached = boot;
      return boot;
    })
    .finally(() => {
      inflight = null;
    });

  return inflight;
}
