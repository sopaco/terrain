import type { AgentExecution } from "./types";

/** Normalize persisted values (legacy `native` → hybrid). */
export function normalizeAgentExecution(
  mode?: AgentExecution | "native" | null,
): AgentExecution {
  if (mode === "acp_native" || mode === "native") return "acp_native";
  return mode ?? "acp";
}

export function isPureAcp(mode?: AgentExecution | "native" | null): boolean {
  return normalizeAgentExecution(mode) === "acp";
}

export function usesNativeLlm(mode?: AgentExecution | "native" | null): boolean {
  return normalizeAgentExecution(mode) === "acp_native";
}

/**
 * Whether human-doc (Litho) generation can run: pure ACP needs the external agent,
 * hybrid mode also accepts a ready native LLM (Litho falls back to it when the
 * configured ACP command cannot run).
 */
export function lithoBackendReady(
  mode: AgentExecution | "native" | null | undefined,
  opts: { acpOk: boolean; llmReady: boolean },
): boolean {
  return isPureAcp(mode) ? opts.acpOk : opts.acpOk || opts.llmReady;
}
