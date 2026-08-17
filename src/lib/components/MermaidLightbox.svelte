<script lang="ts">
  import { copySvgAsImage } from "../clipboard";
  import { tr } from "../i18n";

  interface Props {
    svg: string;
    onclose: () => void;
  }

  let { svg, onclose }: Props = $props();

  const DEFAULT_SCALE = 3;

  let scale = $state(DEFAULT_SCALE);
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
    scale = DEFAULT_SCALE;
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
        mode === "image"
          ? tr("misc.mermaid.copiedImage")
          : tr("misc.mermaid.copiedSvg");
    } catch (e) {
      copyStatus = tr("misc.mermaid.copyFailed", { error: String(e) });
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
  <header class="flex shrink-0 items-center gap-2 border-b border-tr-border-strong px-4 py-3">
    <span class="text-sm font-medium text-tr-ink-2">{tr("misc.mermaid.title")}</span>
    <div class="flex-1"></div>
    <button
      type="button"
      class="tr-press rounded-lg border border-tr-border-strong px-2.5 py-1 text-xs transition-colors hover:bg-tr-elevated"
      onclick={zoomOut}
      title={tr("misc.mermaid.zoomOut")}
    >
      −
    </button>
    <span class="min-w-[3rem] text-center text-xs text-tr-ink-3">{Math.round(scale * 100)}%</span>
    <button
      type="button"
      class="tr-press rounded-lg border border-tr-border-strong px-2.5 py-1 text-xs transition-colors hover:bg-tr-elevated"
      onclick={zoomIn}
      title={tr("misc.mermaid.zoomIn")}
    >
      +
    </button>
    <button
      type="button"
      class="tr-press rounded-lg border border-tr-border-strong px-2.5 py-1 text-xs transition-colors hover:bg-tr-elevated"
      onclick={resetView}
      title={tr("misc.mermaid.resetView")}
    >
      {tr("common.reset")}
    </button>
    <button
      type="button"
      class="tr-press rounded-lg border border-tr-border-strong px-2.5 py-1 text-xs transition-colors hover:bg-tr-elevated disabled:opacity-50"
      disabled={copying}
      onclick={copyImage}
    >
      {copying ? tr("misc.mermaid.copying") : tr("misc.mermaid.copyImage")}
    </button>
    <button
      type="button"
      class="tr-press rounded-lg border border-tr-border-strong px-3 py-1 text-xs transition-colors hover:bg-tr-elevated"
      onclick={onclose}
    >
      {tr("common.close")}
    </button>
  </header>

  {#if copyStatus}
    <p class="shrink-0 border-b border-tr-border px-4 py-1.5 text-xs text-tr-ink-2">{copyStatus}</p>
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
      class="absolute left-1/2 top-1/2 origin-center {dragging ? '' : 'transition-transform duration-150 ease-out'}"
      style={`transform: translate(calc(-50% + ${offsetX}px), calc(-50% + ${offsetY}px)) scale(${scale});`}
    >
      {@html svg}
    </div>
  </div>

  <p class="shrink-0 border-t border-tr-border-strong px-4 py-2 text-center text-xs text-tr-ink-3">
    Drag to pan · Scroll to zoom · Esc to close
  </p>
</div>
