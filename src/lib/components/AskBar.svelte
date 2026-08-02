<script lang="ts">
  import { shouldSubmitOnEnter } from "../ime";
  import { parseAskSlashCommand } from "../askSlashCommands";
  import { UI_MESSAGES } from "../terminology";

  interface Props {
    disabled?: boolean;
    disabledReason?: string | null;
    placeholder?: string;
    onask: (question: string) => void;
    onopen?: () => void;
    onclear?: () => void;
  }

  let {
    disabled = false,
    disabledReason = null,
    placeholder = UI_MESSAGES.askPlaceholder,
    onask,
    onopen,
    onclear,
  }: Props = $props();

  let input = $state("");
  let composing = $state(false);

  function submit() {
    if (composing) return;
    const q = input.trim();
    if (!q || disabled) return;
    if (parseAskSlashCommand(q)?.type === "new") {
      onclear?.();
      input = "";
      return;
    }
    onask(q);
    input = "";
  }

  function onAskButtonClick() {
    if (composing || disabled) return;
    const q = input.trim();
    if (!q) {
      onopen?.();
      return;
    }
    submit();
  }

  function onKeydown(e: KeyboardEvent) {
    if (shouldSubmitOnEnter(e)) {
      e.preventDefault();
      submit();
    }
  }
</script>

<div class="border-t border-tr-border-strong bg-tr-page/95 px-4 py-4 backdrop-blur-sm">
  <div class="mx-auto max-w-3xl">
    <div
      class={`flex items-stretch gap-2 rounded-2xl border bg-tr-surface p-1.5 shadow-lg transition-[border-color,box-shadow] duration-200 ${
        disabled
          ? "border-tr-border-strong opacity-60"
          : "border-tr-border-strong focus-within:border-tr-accent focus-within:ring-2 focus-within:ring-tr-accent-soft-strong"
      }`}
    >
      <textarea
        class="min-h-[44px] flex-1 resize-none bg-transparent px-3 py-2.5 text-sm leading-relaxed outline-none placeholder:text-tr-ink-3 disabled:cursor-not-allowed"
        rows="1"
        {placeholder}
        bind:value={input}
        onkeydown={onKeydown}
        oncompositionstart={() => (composing = true)}
        oncompositionend={() => (composing = false)}
        {disabled}
      ></textarea>
      <button
        type="button"
        class="tr-press my-0.5 shrink-0 self-stretch rounded-xl bg-tr-accent px-5 text-sm font-medium text-white transition-colors hover:bg-tr-accent-hover disabled:cursor-not-allowed disabled:opacity-50"
        {disabled}
        onclick={onAskButtonClick}
      >
        Ask
      </button>
    </div>
    {#if disabled && disabledReason}
      <p class="mt-2 text-center text-xs text-tr-ink-3">{disabledReason}</p>
    {:else}
      <p class="mt-2 text-center text-[11px] text-tr-ink-3">
        Enter 发送 · Shift+Enter 换行 · 中文输入法选词后再按 Enter
      </p>
    {/if}
  </div>
</div>
