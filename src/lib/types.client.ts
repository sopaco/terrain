/**
 * Client-only and UI-layer types (not generated from Rust).
 * IPC types live in `./generated/` — run `bun run gen:types` after Rust schema changes.
 */

import type {
  AskKnowledgeReply,
  IpcSourceSlice,
  SourceCitation,
  TokenUsage,
  ToolCallRecord,
} from "./generated";

/** Knowledge document returned by `read_document` IPC. */
export interface KnowledgeDoc {
  path: string;
  frontmatter: Record<string, unknown>;
  body: string;
}

/** @deprecated legacy value — maps to `acp_native` on load */
export type LegacyAgentExecution = "native";

/** @deprecated use AgentExecution from generated */
export type AskExecution = import("./generated").AgentExecution;

/** Rust `SourceSlice` plus client-side view hints. */
export type SourceSlice = IpcSourceSlice & {
  format?: "code" | "markdown";
  /** Scroll to this line when opening a full-file view from a citation. */
  focus_line?: number;
  /** Client-side load state for optimistic source panel open. */
  status?: "loading" | "ready" | "error";
};

export type AssistantStep =
  | { kind: "text"; content: string }
  | { kind: "tools"; toolCalls: ToolCallRecord[] };

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
  citations?: SourceCitation[];
  toolCalls?: ToolCallRecord[];
  steps?: AssistantStep[];
  timestamp?: number;
  usage?: TokenUsage;
}

export type AppTab = "overview" | "knowledge" | "sdd" | "env";

/** Normalize IPC reply timestamps for UI code expecting `number`. */
export type AskKnowledgeReplyUi = Omit<AskKnowledgeReply, "completed_at"> & {
  completed_at: number;
};

export function normalizeAskReply(reply: AskKnowledgeReply): AskKnowledgeReplyUi {
  return {
    ...reply,
    completed_at: Number(reply.completed_at),
  };
}
