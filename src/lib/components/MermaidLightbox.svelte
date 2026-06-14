<script lang="ts">
  import { copySvgAsImage } from "../clipboard";

  interface Props {
    svg: string;
    onclose: () => void;
  }

  let { svg, onclose }: Props = $props();

  let scale = $state(1);
  let offsetX = $state(0);
  let offsetY = $state(0);
  let dragging = $state(false);
  let dragStartX = 0;
  let dragStartY = 0;
  let copyStatus = $state<string | null>(null);
  let copying = $state(false);

  function zoomIn() {
    scale = Math.min(scale * 1.2, 6);
  }

  function zoomOut() {
    scale = Math.max(scale / 1.2, 0.25);
  }

  function resetView() {
    scale = 1;
    offsetX = 0;
    offsetY = 0;
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    dragging = true;
    dragStartX = e.clientX - offsetX;
    dragStartY = e.clientY - offsetY;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    offsetX = e.clientX - dragStartX;
    offsetY = e.clientY - dragStartY;
  }

  function onPointerUp(e: PointerEvent) {
    dragging = false;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    const delta = e.deltaY > 0 ? 0.9 : 1.1;
    scale = Math.min(Math.max(scale * delta, 0.25), 6);
  }

  async function copyImage() {
    if (copying) return;
    copying = true;
    copyStatus = null;
    try {
      const mode = await copySvgAsImage(svg);
      copyStatus =
        mode === "image" ? "Image copied to clipboard" : "SVG source copied to clipboard";
    } catch (e) {
      copyStatus = `Copy failed: ${e}`;
    } finally {
      copying = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
    if (e.key === "+" || e.key === "=") zoomIn();
    if (e.key === "-") zoomOut();
    if (e.key === "0") resetView();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div
  class="fixed inset-0 z-[60] flex flex-col bg-black/80 backdrop-blur-sm"
  onclick={(e) => e.target === e.currentTarget && onclose()}
  role="presentation"
>
  <header class="flex shrink-0 items-center gap-2 border-b border-white/10 px-4 py-3">
    <span class="text-sm font-medium text-white/80">Mermaid diagram</span>
    <div class="flex-1"></div>
    <button
      type="button"
      class="rounded-lg border border-white/10 px-2.5 py-1 text-xs hover:bg-white/5"
      onclick={zoomOut}
      title="Zoom out (-)"
    >
      −
    </button>
    <span class="min-w-[3rem] text-center text-xs text-white/50">{Math.round(scale * 100)}%</span>
    <button
      type="button"
      class="rounded-lg border border-white/10 px-2.5 py-1 text-xs hover:bg-white/5"
      onclick={zoomIn}
      title="Zoom in (+)"
    >
      +
    </button>
    <button
      type="button"
      class="rounded-lg border border-white/10 px-2.5 py-1 text-xs hover:bg-white/5"
      onclick={resetView}
      title="Reset view (0)"
    >
      Reset
    </button>
    <button
      type="button"
      class="rounded-lg border border-white/10 px-2.5 py-1 text-xs hover:bg-white/5 disabled:opacity-50"
      disabled={copying}
      onclick={copyImage}
    >
      {copying ? "Copying…" : "Copy image"}
    </button>
    <button
      type="button"
      class="rounded-lg border border-white/10 px-3 py-1 text-xs hover:bg-white/5"
      onclick={onclose}
    >
      Close
    </button>
  </header>

  {#if copyStatus}
    <p class="shrink-0 border-b border-white/5 px-4 py-1.5 text-xs text-white/60">{copyStatus}</p>
  {/if}

  <div
    class="relative min-h-0 flex-1 cursor-grab overflow-hidden active:cursor-grabbing"
    onpointerdown={onPointerDown}
    onpointermove={onPointerMove}
    onpointerup={onPointerUp}
    onpointercancel={onPointerUp}
    onwheel={onWheel}
    role="presentation"
  >
    <div
      class="absolute left-1/2 top-1/2 origin-center"
      style={`transform: translate(calc(-50% + ${offsetX}px), calc(-50% + ${offsetY}px)) scale(${scale});`}
    >
      {@html svg}
    </div>
  </div>

  <p class="shrink-0 border-t border-white/10 px-4 py-2 text-center text-xs text-white/40">
    Drag to pan · Scroll to zoom · Esc to close
  </p>
</div>
