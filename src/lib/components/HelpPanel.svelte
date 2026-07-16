<script lang="ts">
  import { GLOSSARY } from "../glossary";
  import CloseButton from "./icons/CloseButton.svelte";
  import ModalShell from "./ModalShell.svelte";

  interface Props {
    open: boolean;
    onclose: () => void;
  }

  let { open, onclose }: Props = $props();
</script>

<ModalShell {open} {onclose} ariaLabelledby="help-title" dialogClass="max-w-[min(92vw,520px)] max-h-[min(80vh,640px)]">
  <header class="flex shrink-0 items-center justify-between border-b border-tr-border-strong px-5 py-4">
    <div>
      <h2 id="help-title" class="text-base font-semibold text-tr-ink">术语说明</h2>
      <p class="mt-0.5 text-xs text-tr-ink-3">Terrain 常用概念速查</p>
    </div>
    <CloseButton onclick={onclose} class="py-1 text-sm" />
  </header>
  <div class="flex-1 overflow-y-auto px-5 py-4">
    <dl class="space-y-4">
      {#each GLOSSARY as entry}
        <div class="rounded-xl border border-tr-border bg-tr-elevated px-4 py-3">
          <dt class="text-sm font-semibold text-tr-accent">{entry.term}</dt>
          <dd class="mt-1.5 text-xs leading-relaxed text-tr-ink-2">{entry.description}</dd>
        </div>
      {/each}
    </dl>
  </div>
  <footer class="shrink-0 border-t border-tr-border-strong px-5 py-3 text-[11px] text-tr-ink-3">
    知识库存放在各仓库的 <span class="font-mono text-tr-ink-3">.terrain/</span> 目录；项目列表登记在
    <span class="font-mono text-tr-ink-3">~/.terrain/registry.json</span>。
  </footer>
</ModalShell>
