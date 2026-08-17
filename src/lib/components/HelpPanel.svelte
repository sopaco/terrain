<script lang="ts">
  import { buildGlossary } from "../glossary";
  import { tr } from "../i18n";
  import CloseButton from "./icons/CloseButton.svelte";
  import ModalShell from "./ModalShell.svelte";

  interface Props {
    open: boolean;
    onclose: () => void;
  }

  let { open, onclose }: Props = $props();

  const glossary = $derived(buildGlossary());
</script>

<ModalShell {open} {onclose} ariaLabelledby="help-title" dialogClass="max-w-[min(92vw,520px)] max-h-[min(80vh,640px)]">
  <header class="flex shrink-0 items-center justify-between border-b border-tr-border-strong px-5 py-4">
    <div>
      <h2 id="help-title" class="text-base font-semibold text-tr-ink">{tr("misc.help.title")}</h2>
      <p class="mt-0.5 text-xs text-tr-ink-3">{tr("misc.help.subtitle")}</p>
    </div>
    <CloseButton onclick={onclose} class="py-1 text-sm" />
  </header>
  <div class="flex-1 overflow-y-auto px-5 py-4">
    <dl class="space-y-4">
      {#each glossary as entry}
        <div class="rounded-xl border border-tr-border bg-tr-elevated px-4 py-3">
          <dt class="text-sm font-semibold text-tr-accent">{entry.term}</dt>
          <dd class="mt-1.5 text-xs leading-relaxed text-tr-ink-2">{entry.description}</dd>
        </div>
      {/each}
    </dl>
  </div>
  <footer class="shrink-0 border-t border-tr-border-strong px-5 py-3 text-[11px] text-tr-ink-3">
    {tr("misc.help.footerKnowledge")} <span class="font-mono text-tr-ink-3">.terrain/</span> {tr("misc.help.footerRegistry")}
    <span class="font-mono text-tr-ink-3">~/.terrain/registry.json</span>{tr("misc.help.footerEnd")}
  </footer>
</ModalShell>
