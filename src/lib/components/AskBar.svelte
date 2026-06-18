<script lang="ts">
  import { shouldSubmitOnEnter } from "../ime";
  import { parseAskSlashCommand } from "../askSlashCommands";

  interface Props {
    disabled?: boolean;
    disabledReason?: string | null;
    placeholder?: string;
    onask: (question: string) => void;
    onclear?: () => void;
  }

  let {
    disabled = false,
    disabledReason = null,
    placeholder = "Ask about this project…",
    onask,
    onclear,
  }: Props = $props();

  let input = $state("");
  let composing = $state(false);

  function submit() {
    if (composing) return;
    const q = input.trim();
    if (!q || disabled) return;
    if (parseAskSlashCommand(q)?.type === "clear") {
      onclear?.();
      input = "";
      return;
    }
    onask(q);
    input = "";
  }

  function onKeydown(e: KeyboardEvent) {
    if (shouldSubmitOnEnter(e)) {
      e.preventDefault();
      submit();
    }
  }
</script>

<div class="border-t border-white/10 bg-[#0f1115]/95 px-4 py-4 backdrop-blur-sm">
  <div class="mx-auto max-w-3xl">
    <div
      class={`flex items-stretch gap-2 rounded-2xl border bg-[#14171c] p-1.5 shadow-lg transition-colors ${
        disabled
          ? "border-white/10 opacity-60"
          : "border-white/10 focus-within:border-indigo-500/70 focus-within:ring-2 focus-within:ring-indigo-500/20"
      }`}
    >
      <textarea
        class="min-h-[44px] flex-1 resize-none bg-transparent px-3 py-2.5 text-sm leading-relaxed outline-none placeholder:text-white/35 disabled:cursor-not-allowed"
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
        class="my-0.5 shrink-0 self-stretch rounded-xl bg-indigo-600 px-5 text-sm font-medium text-white transition-colors hover:bg-indigo-500 disabled:cursor-not-allowed disabled:opacity-50"
        {disabled}
        onclick={submit}
      >
        Ask
      </button>
    </div>
    {#if disabled && disabledReason}
      <p class="mt-2 text-center text-xs text-white/35">{disabledReason}</p>
    {:else}
      <p class="mt-2 text-center text-[11px] text-white/30">
        Enter 发送 · Shift+Enter 换行 · 中文输入法选词后再按 Enter
      </p>
    {/if}
  </div>
</div>
