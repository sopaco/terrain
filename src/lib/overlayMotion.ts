import { tick } from "svelte";

export const OVERLAY_TRANSFORM_MS = 280;
export const OVERLAY_BACKDROP_MS = 200;

export function prefersReducedMotion(): boolean {
  return globalThis.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
}

/** Wait for DOM, then flip the presented class on the next frame (compositor-friendly). */
export async function schedulePresent(setPresented: (value: boolean) => void) {
  await tick();
  if (prefersReducedMotion()) {
    setPresented(true);
    return;
  }
  requestAnimationFrame(() => setPresented(true));
}

export function handleDismissTransitionEnd(
  e: TransitionEvent,
  presented: boolean,
  onHidden: () => void,
  properties: string[] = ["transform"],
) {
  if (e.target !== e.currentTarget || !properties.includes(e.propertyName)) return;
  if (!presented) onHidden();
}
