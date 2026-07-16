<script lang="ts">
  import { checkAcp, checkLlm, getModelSettings, saveModelSettings } from "../api";
  import { isPureAcp, normalizeAgentExecution } from "../agentExecution";
  import type { AgentExecution, AcpSettings, LlmStatus, ModelSettings, ProviderProfile } from "../types";
  import ModalShell from "./ModalShell.svelte";
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
  let acpBinary = $state("opencode");
  let acpArgs = $state("acp");
  let acpCommand = $state("");
  let agentExecution = $state<AgentExecution>("acp");
  let acpTestOk = $state<boolean | null>(null);
  let llmTestOk = $state<boolean | null>(null);
  let llmTestDetail = $state<string | null>(null);

  const pureAcp = $derived(isPureAcp(agentExecution));

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

  function profileToDraft(profile: ProviderProfile | null | undefined, id: ProviderId): ProviderDraft {
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
      model: draft.model.trim() || null,
      api_key: draft.api_key.trim() || null,
      base_url: draft.base_url.trim() || null,
      ollama_host: draft.ollama_host.trim() || null,
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
    agentExecution = normalizeAgentExecution(acp.agent_execution);
  }

  $effect(() => {
    if (!open) return;
    void (async () => {
      error = null;
      acpTestOk = null;
      llmTestOk = null;
      llmTestDetail = null;
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
      binary: acpBinary.trim() || null,
      args: acpArgs.trim() || null,
      command: acpCommand.trim() || null,
      agent_execution: agentExecution,
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
    error = null;
    try {
      acpTestOk = await checkAcp();
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
    try {
      const status = await saveModelSettings(buildSettings());
      onsaved(status);
      if (closeAfter) {
        onclose();
        return;
      }
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
    error = null;
    try {
      const status = await checkLlm();
      llmTestOk = status.ready;
      llmTestDetail = status.ready ? null : status.message;
      onsaved(status);
    } catch (e) {
      error = String(e);
      llmTestOk = false;
      llmTestDetail = null;
    } finally {
      saving = false;
    }
  }
</script>

<ModalShell {open} {onclose} dialogClass="max-w-[min(560px,92vw)] bg-tr-surface">
  <header class="flex items-center justify-between border-b border-tr-border-strong px-5 py-4">
      <div>
        <h2 class="text-base font-semibold">设置</h2>
        <p class="text-xs text-tr-ink-3">ACP 代理与执行模式（保存至 ~/.terrain/settings.json）</p>
      </div>
      <button
        type="button"
        class="rounded-lg border border-tr-border-strong px-3 py-1.5 text-sm text-tr-ink-2 hover:bg-tr-elevated"
        onclick={onclose}
      >
        关闭
      </button>
    </header>

    <div class="flex-1 space-y-4 overflow-y-auto px-5 py-4">
      <label class="block space-y-1.5">
        <span class="text-xs font-medium text-tr-ink-2">执行模式</span>
        <select
          class="w-full rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-2 text-sm outline-none focus:border-tr-accent"
          value={agentExecution}
          onchange={(e) =>
            (agentExecution = (e.currentTarget as HTMLSelectElement).value as AgentExecution)}
        >
          <option value="acp">纯ACP模式</option>
          <option value="acp_native">Native LLM（BYOK） + ACP</option>
        </select>
      </label>

      <div
        class="rounded-lg border border-l-2 border-tr-accent-soft border-l-tr-accent bg-tr-accent-soft px-3.5 py-2.5 text-[11px] leading-[1.65] text-tr-ink-2"
        role="note"
      >
        {#if pureAcp}
          <p>
            问答、Litho、SDD、Agent 上下文等全部由外部 ACP 代理处理，只需配置下方 ACP 命令，无需填写
            Native LLM。默认 <code class="rounded bg-tr-page px-1 py-0.5 text-tr-accent">opencode acp</code>。
          </p>
        {:else}
          <p>
            Native LLM（BYOK）处理问答、SDD 文档阶段与 Agent Context，支持流式输出与工具调用详情；ACP 处理 Litho 与 SDD 代码生成。请同时配置下方两项。
          </p>
        {/if}
      </div>

      <div class="space-y-3 rounded-xl border border-tr-border-strong bg-tr-elevated p-4">
        <h3 class="text-sm font-medium text-tr-ink-2">ACP 代理</h3>

        <label class="block space-y-1.5">
          <span class="text-xs font-medium text-tr-ink-2">Binary（PATH 上的可执行文件）</span>
          <input
            class="w-full rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-2 text-sm outline-none focus:border-tr-accent"
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
          <span class="text-xs font-medium text-tr-ink-2">参数（跟在 binary 后）</span>
          <input
            class="w-full rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-2 text-sm outline-none focus:border-tr-accent"
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
          <span class="text-xs font-medium text-tr-ink-2">完整命令覆盖（可选，优先于 binary + args）</span>
          <input
            class="w-full rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-2 text-sm outline-none focus:border-tr-accent"
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

        <button
          type="button"
          class="w-full rounded-lg border border-tr-border-strong py-2 text-xs hover:bg-tr-elevated disabled:opacity-50"
          disabled={saving}
          onclick={testAcp}
        >
          检测 ACP 代理
        </button>
        {#if acpTestOk === true}
          <p class="text-[11px] text-tr-good">检测通过</p>
        {:else if acpTestOk === false}
          <p class="text-[11px] text-tr-watch">未检测到，请检查 binary 或 command</p>
        {/if}
      </div>

      {#if !pureAcp}
        <div class="space-y-3 rounded-xl border border-tr-accent-soft-strong bg-tr-accent-soft p-4">
          <h3 class="text-sm font-medium text-tr-ink-2">Native LLM</h3>

          <label class="block space-y-1.5">
            <span class="text-xs font-medium text-tr-ink-2">Provider</span>
            <select
              class="w-full rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-2 text-sm outline-none focus:border-tr-accent"
              value={provider}
              onchange={(e) => (provider = (e.currentTarget as HTMLSelectElement).value as ProviderId)}
            >
              {#each providerOptions as opt}
                <option value={opt.id}>{opt.label}</option>
              {/each}
            </select>
          </label>

          <label class="block space-y-1.5">
            <span class="text-xs font-medium text-tr-ink-2">Model</span>
            <input
              class="w-full rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-2 text-sm outline-none focus:border-tr-accent"
              value={current.model}
              oninput={(e) => patchCurrent({ model: (e.currentTarget as HTMLInputElement).value })}
              placeholder="e.g. stepfun-ai/step-3.7-flash"
            />
          </label>

          {#if provider !== "ollama"}
            <label class="block space-y-1.5">
              <span class="text-xs font-medium text-tr-ink-2">API Key</span>
              <input
                type="password"
                class="w-full rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-2 text-sm outline-none focus:border-tr-accent"
                value={current.api_key}
                oninput={(e) => patchCurrent({ api_key: (e.currentTarget as HTMLInputElement).value })}
                placeholder={provider === "lmstudio" ? "lm-studio" : "nvapi-…"}
                autocomplete="off"
              />
            </label>

            <label class="block space-y-1.5">
              <span class="text-xs font-medium text-tr-ink-2">Base URL</span>
              <input
                class="w-full rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-2 text-sm outline-none focus:border-tr-accent"
                value={current.base_url}
                oninput={(e) => patchCurrent({ base_url: (e.currentTarget as HTMLInputElement).value })}
                placeholder="https://integrate.api.nvidia.com/v1"
              />
            </label>
          {/if}

          {#if provider === "ollama"}
            <label class="block space-y-1.5">
              <span class="text-xs font-medium text-tr-ink-2">Ollama Host</span>
              <input
                class="w-full rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-2 text-sm outline-none focus:border-tr-accent"
                value={current.ollama_host}
                oninput={(e) =>
                  patchCurrent({ ollama_host: (e.currentTarget as HTMLInputElement).value })}
                placeholder="http://localhost:11434"
              />
            </label>
          {/if}

          <button
            type="button"
            class="w-full rounded-lg border border-tr-border-strong py-2 text-xs hover:bg-tr-elevated disabled:opacity-50"
            disabled={saving}
            onclick={testConnection}
          >
            测试 LLM 连接
          </button>
          {#if llmTestOk === true}
            <p class="text-[11px] text-tr-good">连接正常</p>
          {:else if llmTestOk === false}
            <p class="text-[11px] text-tr-watch">{llmTestDetail ?? "连接失败，请检查配置"}</p>
          {/if}
        </div>
      {/if}

      <p class="text-[11px] leading-relaxed text-tr-ink-3">
        每个 Provider 的配置会分别保存到 ~/.terrain/settings.json。
        仅在没有设置文件时，才会读取 `.env` 中的默认值。
      </p>

      {#if error}
        <p class="rounded-lg border border-tr-critical/30 bg-tr-critical-soft px-3 py-2 text-xs text-tr-critical">{error}</p>
      {/if}
    </div>

    <footer class="border-t border-tr-border-strong px-5 py-4">
      <button
        type="button"
        class="w-full rounded-xl bg-tr-accent py-2.5 text-sm font-medium hover:bg-tr-accent-hover disabled:opacity-50"
        disabled={saving}
        onclick={() => save(true)}
      >
        {saving ? "保存中…" : "保存"}
      </button>
    </footer>
</ModalShell>
