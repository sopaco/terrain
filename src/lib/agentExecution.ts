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
