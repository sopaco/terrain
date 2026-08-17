<script lang="ts">
  import { X } from "@lucide/svelte";
  import { tr } from "../../i18n";

  interface Props {
    onclick: () => void;
    class?: string;
    size?: number;
    label?: string;
  }

  let {
    onclick,
    class: className = "",
    size = 14,
    label,
  }: Props = $props();

  const closeLabel = $derived(label ?? tr("common.close"));
</script>

<button
  type="button"
  class="tr-press inline-flex shrink-0 items-center justify-center rounded-lg border border-tr-border-strong px-2.5 py-1 text-tr-ink-2 transition-colors hover:bg-tr-elevated {className}"
  {onclick}
  aria-label={closeLabel}
>
  <X {size} strokeWidth={2} aria-hidden="true" />
</button>
