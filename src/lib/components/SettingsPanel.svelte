<script lang="ts">
  import { checkAcp, checkLlm, getModelSettings, saveModelSettings } from "../api";
  import { isPureAcp, normalizeAgentExecution } from "../agentExecution";
  import type {
    AgentExecution,
    AcpSettings,
    KnowledgeSettings,
    LanguageSetting,
    LlmStatus,
    ModelSettings,
    ProviderProfile,
  } from "../types";
  import { applyLocale, tr, t } from "../i18n";
  import { setStatus } from "../stores/status.svelte";
  import ModalShell from "./ModalShell.svelte";
  import {
    DEFAULT_INCREMENTAL_MAX_CHANGED_FILES,
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
  let incrementalRefresh = $state(true);
  let incrementalMaxChangedFiles = $state(DEFAULT_INCREMENTAL_MAX_CHANGED_FILES);
  let incrementalHumanDocs = $state(false);
  let language = $state<LanguageSetting>("system");
  let savedLanguage = $state<LanguageSetting>("system");
  let acpTestOk = $state<boolean | null>(null);
  let llmTestOk = $state<boolean | null>(null);
  let llmTestDetail = $state<string | null>(null);

  const pureAcp = $derived(isPureAcp(agentExecution));

  const providerOptions = [
    { id: "openai" as const },
    { id: "lmstudio" as const },
    { id: "ollama" as const },
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
    const knowledge = s.knowledge;
    incrementalRefresh = knowledge?.incremental_refresh ?? true;
    incrementalMaxChangedFiles =
      knowledge?.incremental_max_changed_files ||
      DEFAULT_INCREMENTAL_MAX_CHANGED_FILES;
    incrementalHumanDocs = knowledge?.incremental_human_docs ?? false;
    language = s.language ?? "system";
    savedLanguage = language;
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
    const knowledge: KnowledgeSettings = {
      incremental_refresh: incrementalRefresh,
      incremental_max_changed_files: Math.max(
        1,
        Math.round(incrementalMaxChangedFiles) ||
          DEFAULT_INCREMENTAL_MAX_CHANGED_FILES,
      ),
      incremental_human_docs: incrementalHumanDocs,
    };
    return {
      provider,
      model: active.model,
      api_key: active.api_key,
      base_url: active.base_url,
      ollama_host: active.ollama_host,
      profiles,
      acp,
      knowledge,
      language,
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
      // Apply the language immediately; if it changed, knowledge assets keep
      // their previous language until the user regenerates them.
      const languageChanged = language !== savedLanguage;
      applyLocale(language);
      savedLanguage = language;
      onsaved(status);
      if (languageChanged) {
        setStatus(t("settings.language.assetsStale"), "success", null, 8000);
      }
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
        <h2 class="text-base font-semibold">{tr("settings.title")}</h2>
        <p class="text-xs text-tr-ink-3">{tr("settings.subtitle")}</p>
      </div>
      <button
        type="button"
        class="tr-press rounded-lg border border-tr-border-strong px-3 py-1.5 text-sm text-tr-ink-2 transition-colors hover:bg-tr-elevated"
        onclick={onclose}
      >
        {tr("common.close")}
      </button>
    </header>

    <div class="flex-1 space-y-4 overflow-y-auto px-5 py-4">
      <label class="block space-y-1.5">
        <span class="text-xs font-medium text-tr-ink-2">{tr("settings.language.label")}</span>
        <select
          class="w-full rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-2 text-sm outline-none focus:border-tr-accent"
          value={language}
          onchange={(e) =>
            (language = (e.currentTarget as HTMLSelectElement).value as LanguageSetting)}
        >
          <option value="system">{tr("settings.language.system")}</option>
          <option value="zh-CN">简体中文</option>
          <option value="en">English</option>
        </select>
        <p class="text-xs text-tr-ink-3">{tr("settings.language.hint")}</p>
      </label>

      <label class="block space-y-1.5">
        <span class="text-xs font-medium text-tr-ink-2">{tr("settings.executionMode.label")}</span>
        <select
          class="w-full rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-2 text-sm outline-none focus:border-tr-accent"
          value={agentExecution}
          onchange={(e) =>
            (agentExecution = (e.currentTarget as HTMLSelectElement).value as AgentExecution)}
        >
          <option value="acp">{tr("settings.executionMode.pureAcp")}</option>
          <option value="acp_native">{tr("settings.executionMode.hybridOption")}</option>
        </select>
      </label>

      <div
        class="rounded-lg border border-l-2 border-tr-accent-soft border-l-tr-accent bg-tr-accent-soft px-3.5 py-2.5 text-[11px] leading-[1.65] text-tr-ink-2"
        role="note"
      >
        {#if pureAcp}
          <p>
            {tr("settings.modeNote.pureAcpBefore")}<code class="rounded bg-tr-page px-1 py-0.5 text-tr-accent">opencode acp</code>{tr("settings.modeNote.pureAcpAfter")}
          </p>
        {:else}
          <p>
            {tr("settings.modeNote.hybrid")}
          </p>
        {/if}
      </div>

      <div class="space-y-3 rounded-xl border border-tr-border-strong bg-tr-elevated p-4">
        <h3 class="text-sm font-medium text-tr-ink-2">{tr("settings.acp.title")}</h3>

        <label class="block space-y-1.5">
          <span class="text-xs font-medium text-tr-ink-2">{tr("settings.acp.binary")}</span>
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
          <span class="text-xs font-medium text-tr-ink-2">{tr("settings.acp.args")}</span>
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
          <span class="text-xs font-medium text-tr-ink-2">{tr("settings.acp.commandOverride")}</span>
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
          class="tr-press w-full rounded-lg border border-tr-border-strong py-2 text-xs transition-colors hover:bg-tr-elevated disabled:opacity-50"
          disabled={saving}
          onclick={testAcp}
        >
          {tr("settings.acp.test")}
        </button>
        {#if acpTestOk === true}
          <p class="text-[11px] text-tr-good">{tr("settings.acp.testOk")}</p>
        {:else if acpTestOk === false}
          <p class="text-[11px] text-tr-watch">{tr("settings.acp.testFailed")}</p>
        {/if}
      </div>

      {#if !pureAcp}
        <div class="space-y-3 rounded-xl border border-tr-accent-soft-strong bg-tr-accent-soft p-4">
          <h3 class="text-sm font-medium text-tr-ink-2">{tr("settings.llm.title")}</h3>

          <label class="block space-y-1.5">
            <span class="text-xs font-medium text-tr-ink-2">Provider</span>
            <select
              class="w-full rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-2 text-sm outline-none focus:border-tr-accent"
              value={provider}
              onchange={(e) => (provider = (e.currentTarget as HTMLSelectElement).value as ProviderId)}
            >
              {#each providerOptions as opt}
                <option value={opt.id}>{tr(`settings.provider.${opt.id}`)}</option>
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
            class="tr-press w-full rounded-lg border border-tr-border-strong py-2 text-xs transition-colors hover:bg-tr-elevated disabled:opacity-50"
            disabled={saving}
            onclick={testConnection}
          >
            {tr("settings.llm.test")}
          </button>
          {#if llmTestOk === true}
            <p class="text-[11px] text-tr-good">{tr("settings.llm.testOk")}</p>
          {:else if llmTestOk === false}
            <p class="text-[11px] text-tr-watch">{llmTestDetail ?? tr("settings.llm.testFailed")}</p>
          {/if}
        </div>
      {/if}

      <div class="space-y-3 rounded-xl border border-tr-border-strong bg-tr-elevated p-4">
        <div>
          <h3 class="text-sm font-medium text-tr-ink-2">{tr("settings.freshness.title")}</h3>
          <p class="mt-0.5 text-[11px] leading-relaxed text-tr-ink-3">
            {tr("settings.freshness.hint")}
          </p>
        </div>

        <label class="flex items-start gap-2.5">
          <input
            class="mt-0.5 accent-tr-accent"
            type="checkbox"
            checked={incrementalRefresh}
            onchange={(e) =>
              (incrementalRefresh = (e.currentTarget as HTMLInputElement).checked)}
          />
          <span class="space-y-0.5">
            <span class="block text-xs font-medium text-tr-ink-2">{tr("settings.freshness.incremental")}</span>
            <span class="block text-[11px] leading-relaxed text-tr-ink-3">
              {tr("settings.freshness.incrementalHint")}
            </span>
          </span>
        </label>

        <label class="block space-y-1.5" class:opacity-50={!incrementalRefresh}>
          <span class="text-xs font-medium text-tr-ink-2">{tr("settings.freshness.maxChangedFiles")}</span>
          <input
            class="w-full rounded-lg border border-tr-border-strong bg-tr-elevated px-3 py-2 text-sm outline-none focus:border-tr-accent"
            type="number"
            min="1"
            max="2000"
            step="1"
            disabled={!incrementalRefresh}
            value={incrementalMaxChangedFiles}
            oninput={(e) =>
              (incrementalMaxChangedFiles = Number(
                (e.currentTarget as HTMLInputElement).value,
              ))}
          />
          <span class="block text-[11px] leading-relaxed text-tr-ink-3">
            {tr("settings.freshness.maxChangedFilesHint", { count: DEFAULT_INCREMENTAL_MAX_CHANGED_FILES })}
          </span>
        </label>

        <label class="flex items-start gap-2.5" class:opacity-50={!incrementalRefresh}>
          <input
            class="mt-0.5 accent-tr-accent"
            type="checkbox"
            disabled={!incrementalRefresh}
            checked={incrementalHumanDocs}
            onchange={(e) =>
              (incrementalHumanDocs = (e.currentTarget as HTMLInputElement).checked)}
          />
          <span class="space-y-0.5">
            <span class="block text-xs font-medium text-tr-ink-2">
              {tr("settings.freshness.syncHumanDocs")}
            </span>
            <span class="block text-[11px] leading-relaxed text-tr-ink-3">
              {tr("settings.freshness.syncHumanDocsHint")}
            </span>
          </span>
        </label>
      </div>

      <p class="text-[11px] leading-relaxed text-tr-ink-3">
        {tr("settings.saveHint")}
      </p>

      {#if error}
        <p class="rounded-lg border border-tr-critical/30 bg-tr-critical-soft px-3 py-2 text-xs text-tr-critical">{error}</p>
      {/if}
    </div>

    <footer class="border-t border-tr-border-strong px-5 py-4">
      <button
        type="button"
        class="tr-press w-full rounded-xl bg-tr-accent py-2.5 text-sm font-medium transition-colors hover:bg-tr-accent-hover disabled:opacity-50"
        disabled={saving}
        onclick={() => save(true)}
      >
        {saving ? tr("common.saving") : tr("common.save")}
      </button>
    </footer>
</ModalShell>
