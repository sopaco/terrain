<script lang="ts">
  import { checkAcp, checkLlm, getModelSettings, saveModelSettings } from "../api";
  import type { AcpSettings, LlmStatus, ModelSettings, ProviderProfile } from "../types";
  import {
    DEFAULT_LMSTUDIO_BASE_URL,
    DEFAULT_LMSTUDIO_MODEL,
    DEFAULT_OLLAMA_HOST,
    DEFAULT_OLLAMA_MODEL,
    DEFAULT_OPENAI_BASE_URL,
    DEFAULT_OPENAI_MODEL,
  } from "../constants";

  type ProviderId = "openai" | "lmstudio" | "ollama";

  type ProviderDraft = {
    model: string;
    api_key: string;
    base_url: string;
    ollama_host: string;
  };

  interface Props {
    open: boolean;
    onclose: () => void;
    onsaved: (status: LlmStatus) => void;
  }

  let { open, onclose, onsaved }: Props = $props();

  const providerIds: ProviderId[] = ["openai", "lmstudio", "ollama"];

  let provider = $state<ProviderId>("openai");
  let drafts = $state<Record<ProviderId, ProviderDraft>>(emptyDrafts());
  let saving = $state(false);
  let error = $state<string | null>(null);
  let statusMessage = $state<string | null>(null);
  let acpBinary = $state("opencode");
  let acpArgs = $state("acp");
  let acpCommand = $state("");
  let askExecution = $state<"native" | "acp">("native");
  let acpTestOk = $state<boolean | null>(null);

  const providerOptions = [
    { id: "openai" as const, label: "OpenAI 兼容 (NVIDIA Integrate 等)" },
    { id: "lmstudio" as const, label: "LM Studio (本地)" },
    { id: "ollama" as const, label: "Ollama (本地)" },
  ];

  const current = $derived(drafts[provider]);

  function emptyDrafts(): Record<ProviderId, ProviderDraft> {
    return {
      openai: defaultDraft("openai"),
      lmstudio: defaultDraft("lmstudio"),
      ollama: defaultDraft("ollama"),
    };
  }

  function defaultDraft(id: ProviderId): ProviderDraft {
    if (id === "lmstudio") {
      return {
        model: DEFAULT_LMSTUDIO_MODEL,
        api_key: "lm-studio",
        base_url: DEFAULT_LMSTUDIO_BASE_URL,
        ollama_host: DEFAULT_OLLAMA_HOST,
      };
    }
    if (id === "ollama") {
      return {
        model: DEFAULT_OLLAMA_MODEL,
        api_key: "",
        base_url: "",
        ollama_host: DEFAULT_OLLAMA_HOST,
      };
    }
    return {
      model: DEFAULT_OPENAI_MODEL,
      api_key: "",
      base_url: DEFAULT_OPENAI_BASE_URL,
      ollama_host: DEFAULT_OLLAMA_HOST,
    };
  }

  function profileToDraft(profile: ProviderProfile | undefined, id: ProviderId): ProviderDraft {
    const base = defaultDraft(id);
    if (!profile) return base;
    return {
      model: profile.model ?? base.model,
      api_key: profile.api_key ?? base.api_key,
      base_url: profile.base_url ?? base.base_url,
      ollama_host: profile.ollama_host ?? base.ollama_host,
    };
  }

  function draftToProfile(draft: ProviderDraft): ProviderProfile {
    return {
      model: draft.model.trim() || undefined,
      api_key: draft.api_key.trim() || undefined,
      base_url: draft.base_url.trim() || undefined,
      ollama_host: draft.ollama_host.trim() || undefined,
    };
  }

  function patchCurrent(patch: Partial<ProviderDraft>) {
    drafts = {
      ...drafts,
      [provider]: { ...drafts[provider], ...patch },
    };
  }

  function loadFromSettings(s: ModelSettings) {
    const next = emptyDrafts();
    for (const id of providerIds) {
      next[id] = profileToDraft(s.profiles?.[id], id);
    }
    const active = (s.provider ?? "openai") as ProviderId;
    if (!s.profiles || Object.keys(s.profiles).length === 0) {
      next[active] = profileToDraft(
        {
          model: s.model,
          api_key: s.api_key,
          base_url: s.base_url,
          ollama_host: s.ollama_host,
        },
        active,
      );
    }
    drafts = next;
    provider = active;
    const acp = s.acp ?? {};
    acpBinary = acp.binary ?? "opencode";
    acpArgs = acp.args ?? "acp";
    acpCommand = acp.command ?? "";
    askExecution = acp.ask_execution ?? "native";
  }

  $effect(() => {
    if (!open) return;
    void (async () => {
      error = null;
      statusMessage = null;
      const s = await getModelSettings();
      loadFromSettings(s);
    })();
  });

  function buildSettings(): ModelSettings {
    const profiles: Record<string, ProviderProfile> = {};
    for (const id of providerIds) {
      profiles[id] = draftToProfile(drafts[id]);
    }
    const active = draftToProfile(drafts[provider]);
    const acp: AcpSettings = {
      binary: acpBinary.trim() || undefined,
      args: acpArgs.trim() || undefined,
      command: acpCommand.trim() || undefined,
      ask_execution: askExecution,
      auto_approve: true,
    };
    return {
      provider,
      model: active.model,
      api_key: active.api_key,
      base_url: active.base_url,
      ollama_host: active.ollama_host,
      profiles,
      acp,
    };
  }

  async function testAcp() {
    await save(false);
    if (error) return;
    saving = true;
    try {
      acpTestOk = await checkAcp();
      statusMessage = acpTestOk
        ? "ACP 代理已在 PATH 中找到"
        : "未找到 ACP 代理，请检查 binary / command 设置";
    } catch (e) {
      error = String(e);
      acpTestOk = false;
    } finally {
      saving = false;
    }
  }

  async function save(closeAfter = true) {
    saving = true;
    error = null;
    statusMessage = null;
    try {
      const status = await saveModelSettings(buildSettings());
      onsaved(status);
      if (closeAfter) {
        onclose();
        return;
      }
      statusMessage = status.ready ? "已保存，LLM 已就绪" : status.message;
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function testConnection() {
    await save(false);
    if (error) return;
    saving = true;
    try {
      const status = await checkLlm();
      statusMessage = status.ready ? `连接正常：${status.message}` : status.message;
      onsaved(status);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onclose();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm" onclick={onclose} role="presentation"></div>

  <section
    class="fixed left-1/2 top-1/2 z-[51] flex max-h-[min(90vh,720px)] w-[min(560px,92vw)] -translate-x-1/2 -translate-y-1/2 flex-col overflow-hidden rounded-2xl border border-white/10 bg-[#161a22] shadow-2xl"
    aria-label="Settings"
  >
    <header class="flex items-center justify-between border-b border-white/10 px-5 py-4">
      <div>
        <h2 class="text-base font-semibold">设置</h2>
        <p class="text-xs text-white/40">大模型、ACP 代理与 Ask 模式（保存至 ~/.mind-mesh/settings.json）</p>
      </div>
      <button
        type="button"
        class="rounded-lg border border-white/10 px-3 py-1.5 text-sm text-white/70 hover:bg-white/5"
        onclick={onclose}
      >
        关闭
      </button>
    </header>

    <div class="flex-1 space-y-4 overflow-y-auto px-5 py-4">
      <label class="block space-y-1.5">
        <span class="text-xs font-medium text-white/55">Provider</span>
        <select
          class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm outline-none focus:border-indigo-500"
          value={provider}
          onchange={(e) => (provider = (e.currentTarget as HTMLSelectElement).value as ProviderId)}
        >
          {#each providerOptions as opt}
            <option value={opt.id}>{opt.label}</option>
          {/each}
        </select>
      </label>

      <label class="block space-y-1.5">
        <span class="text-xs font-medium text-white/55">Model</span>
        <input
          class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm outline-none focus:border-indigo-500"
          value={current.model}
          oninput={(e) => patchCurrent({ model: (e.currentTarget as HTMLInputElement).value })}
          placeholder="e.g. stepfun-ai/step-3.7-flash"
        />
      </label>

      {#if provider !== "ollama"}
        <label class="block space-y-1.5">
          <span class="text-xs font-medium text-white/55">API Key</span>
          <input
            type="password"
            class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm outline-none focus:border-indigo-500"
            value={current.api_key}
            oninput={(e) => patchCurrent({ api_key: (e.currentTarget as HTMLInputElement).value })}
            placeholder={provider === "lmstudio" ? "lm-studio" : "nvapi-…"}
            autocomplete="off"
          />
        </label>

        <label class="block space-y-1.5">
          <span class="text-xs font-medium text-white/55">Base URL</span>
          <input
            class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm outline-none focus:border-indigo-500"
            value={current.base_url}
            oninput={(e) => patchCurrent({ base_url: (e.currentTarget as HTMLInputElement).value })}
            placeholder="https://integrate.api.nvidia.com/v1"
          />
        </label>
      {/if}

      {#if provider === "ollama"}
        <label class="block space-y-1.5">
          <span class="text-xs font-medium text-white/55">Ollama Host</span>
          <input
            class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm outline-none focus:border-indigo-500"
            value={current.ollama_host}
            oninput={(e) =>
              patchCurrent({ ollama_host: (e.currentTarget as HTMLInputElement).value })}
            placeholder="http://localhost:11434"
          />
        </label>
      {/if}

      <div class="space-y-3 rounded-xl border border-white/10 bg-white/[0.02] p-4">
        <h3 class="text-sm font-medium text-white/80">ACP 代理</h3>
        <p class="text-[11px] leading-relaxed text-white/35">
          用于 Litho、SDD 代码生成，以及 Ask 的 ACP 模式。默认 <code class="text-white/50">opencode acp</code>，可改为任意支持 ACP 的 CLI。
        </p>

        <label class="block space-y-1.5">
          <span class="text-xs font-medium text-white/55">Binary（PATH 上的可执行文件）</span>
          <input
            class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm outline-none focus:border-indigo-500"
            type="text"
            autocapitalize="off"
            autocorrect="off"
            spellcheck={false}
            autocomplete="off"
            value={acpBinary}
            oninput={(e) => (acpBinary = (e.currentTarget as HTMLInputElement).value)}
            placeholder="opencode"
          />
        </label>

        <label class="block space-y-1.5">
          <span class="text-xs font-medium text-white/55">参数（跟在 binary 后）</span>
          <input
            class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm outline-none focus:border-indigo-500"
            type="text"
            autocapitalize="off"
            autocorrect="off"
            spellcheck={false}
            autocomplete="off"
            value={acpArgs}
            oninput={(e) => (acpArgs = (e.currentTarget as HTMLInputElement).value)}
            placeholder="acp"
          />
        </label>

        <label class="block space-y-1.5">
          <span class="text-xs font-medium text-white/55">完整命令覆盖（可选，优先于 binary + args）</span>
          <input
            class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm outline-none focus:border-indigo-500"
            type="text"
            autocapitalize="off"
            autocorrect="off"
            spellcheck={false}
            autocomplete="off"
            value={acpCommand}
            oninput={(e) => (acpCommand = (e.currentTarget as HTMLInputElement).value)}
            placeholder="opencode acp"
          />
        </label>

        <label class="block space-y-1.5">
          <span class="text-xs font-medium text-white/55">Ask 执行模式</span>
          <select
            class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-sm outline-none focus:border-indigo-500"
            value={askExecution}
            onchange={(e) =>
              (askExecution = (e.currentTarget as HTMLSelectElement).value as "native" | "acp")}
          >
            <option value="native">Native — 内置 LLM 工具调用</option>
            <option value="acp">ACP — 外部代理 + mind-mesh CLI（见 mind-mesh-ask-skill）</option>
          </select>
        </label>

        <button
          type="button"
          class="w-full rounded-lg border border-white/10 py-2 text-xs hover:bg-white/5 disabled:opacity-50"
          disabled={saving}
          onclick={testAcp}
        >
          检测 ACP 代理
        </button>
        {#if acpTestOk === true}
          <p class="text-[11px] text-emerald-300/80">ACP 代理可用</p>
        {:else if acpTestOk === false}
          <p class="text-[11px] text-amber-300/80">ACP 代理未检测到</p>
        {/if}
      </div>

      <p class="text-[11px] leading-relaxed text-white/35">
        每个 Provider 的配置会分别保存到 ~/.mind-mesh/settings.json，并优先生效。
        仅在没有设置文件时，才会读取 `.env` 中的默认值。
      </p>

      {#if error}
        <p class="rounded-lg border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-xs text-rose-200">{error}</p>
      {/if}
      {#if statusMessage}
        <p class="rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-xs text-emerald-200">
          {statusMessage}
        </p>
      {/if}
    </div>

    <footer class="flex gap-2 border-t border-white/10 px-5 py-4">
      <button
        type="button"
        class="flex-1 rounded-xl border border-white/10 py-2.5 text-sm hover:bg-white/5 disabled:opacity-50"
        disabled={saving}
        onclick={testConnection}
      >
        测试连接
      </button>
      <button
        type="button"
        class="flex-1 rounded-xl bg-indigo-600 py-2.5 text-sm font-medium hover:bg-indigo-500 disabled:opacity-50"
        disabled={saving}
        onclick={() => save(true)}
      >
        {saving ? "保存中…" : "保存"}
      </button>
    </footer>
  </section>
{/if}
