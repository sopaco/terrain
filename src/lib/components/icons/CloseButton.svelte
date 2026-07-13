<script lang="ts">
  import { X } from "@lucide/svelte";

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
    label = "关闭",
  }: Props = $props();
</script>

<button
  type="button"
  class="inline-flex shrink-0 items-center justify-center rounded-lg border border-white/10 px-2.5 py-1 text-white/60 hover:bg-white/5 {className}"
  {onclick}
  aria-label={label}
>
  <X {size} strokeWidth={2} aria-hidden="true" />
</button>
