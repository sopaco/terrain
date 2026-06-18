<script lang="ts">
  import { GLOSSARY } from "../glossary";
  import ModalShell from "./ModalShell.svelte";

  interface Props {
    open: boolean;
    onclose: () => void;
  }

  let { open, onclose }: Props = $props();
</script>

<ModalShell {open} {onclose} ariaLabelledby="help-title" dialogClass="max-w-[min(92vw,520px)] max-h-[min(80vh,640px)]">
  <header class="flex shrink-0 items-center justify-between border-b border-white/10 px-5 py-4">
    <div>
      <h2 id="help-title" class="text-base font-semibold text-white/95">术语说明</h2>
      <p class="mt-0.5 text-xs text-white/45">MindMesh 常用概念速查</p>
    </div>
    <button
      type="button"
      class="rounded-lg border border-white/10 px-2.5 py-1 text-sm text-white/60 hover:bg-white/5"
      onclick={onclose}
      aria-label="关闭"
    >
      ✕
    </button>
  </header>
  <div class="flex-1 overflow-y-auto px-5 py-4">
    <dl class="space-y-4">
      {#each GLOSSARY as entry}
        <div class="rounded-xl border border-white/8 bg-white/[0.02] px-4 py-3">
          <dt class="text-sm font-semibold text-indigo-200/95">{entry.term}</dt>
          <dd class="mt-1.5 text-xs leading-relaxed text-white/55">{entry.description}</dd>
        </div>
      {/each}
    </dl>
  </div>
  <footer class="shrink-0 border-t border-white/10 px-5 py-3 text-[11px] text-white/35">
    知识库存放在各仓库的 <span class="font-mono text-white/50">.mind-mesh/</span> 目录；项目列表登记在
    <span class="font-mono text-white/50">~/.mind-mesh/registry.json</span>。
  </footer>
</ModalShell>
