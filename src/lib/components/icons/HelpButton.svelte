<script lang="ts">
  import { CircleQuestionMark } from "@lucide/svelte";

  type Variant = "icon" | "toolbar";

  interface Props {
    onclick: () => void;
    title?: string;
    ariaLabel?: string;
    class?: string;
    size?: number;
    strokeWidth?: number;
    variant?: Variant;
  }

  let {
    onclick,
    title,
    ariaLabel = "帮助说明",
    class: className = "",
    size,
    strokeWidth,
    variant = "icon",
  }: Props = $props();

  const variantClass: Record<Variant, string> = {
    icon: "inline-flex items-center justify-center text-white/50 transition-colors hover:text-indigo-200",
    toolbar:
      "inline-flex shrink-0 items-center justify-center rounded-lg border border-white/10 px-2.5 py-1.5 text-white/70 transition-colors hover:bg-white/5 hover:text-indigo-200",
  };

  const iconSize = $derived(size ?? (variant === "toolbar" ? 16 : 15));
  const iconStroke = $derived(strokeWidth ?? (iconSize >= 22 ? 2.25 : 2));
</script>

<button
  type="button"
  class="{variantClass[variant]} {className}"
  {title}
  aria-label={ariaLabel}
  {onclick}
>
  <CircleQuestionMark size={iconSize} strokeWidth={iconStroke} aria-hidden="true" />
</button>
