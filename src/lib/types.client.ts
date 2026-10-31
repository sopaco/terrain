/**
 * Client-only and UI-layer types (not generated from Rust).
 * IPC types live in `./generated/` — run `bun run gen:types` after Rust schema changes.
 */

import type {
  AskKnowledgeReply,
  ChatPhase,
  IpcSourceSlice,
  SourceCitation,
  TokenUsage,
  ToolCallRecord,
} from "./generated";

/** Streaming payloads from `ask_knowledge_cmd` via Tauri Channel (mirrors Rust `AskStreamEvent`). */
export type AskStreamEvent =
  | { type: "chunk"; text: string }
  | { type: "thinking_chunk"; text: string }
  | { type: "tool_calls"; tool_calls: ToolCallRecord[] }
  | { type: "phase"; phase: ChatPhase }
  | { type: "usage"; usage: TokenUsage }
  | { type: "done"; reply: AskKnowledgeReply };

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
  | { kind: "thinking"; content: string }
  | { kind: "text"; content: string }
  | { kind: "tools"; toolCalls: ToolCallRecord[] };

export interface ChatMessage {
  role: "user" | "assistant";
  content: string;
  citations?: SourceCitation[];
  toolCalls?: ToolCallRecord[];
  steps?: AssistantStep[];
  timestamp?: number;
  errorDetail?: string | null;
  isError?: boolean;
  usage?: TokenUsage;
}

export type AppTab =
  | "dashboard"
  | "overview"
  | "knowledge"
  | "graph"
  | "git"
  | "sdd"
  | "env";

/** Mirror of the Rust `GraphifyStatus` command payload (src-tauri commands/graphify.rs). */
export interface GraphifyCommunity {
  name: string;
  size: number;
}

export interface GraphifyStatus {
  binary_available: boolean;
  binary_path: string;
  out_dir: string;
  graph_present: boolean;
  report_present: boolean;
  html_present: boolean;
  wiki_present: boolean;
  graph_mtime_secs: number | null;
  nodes: number | null;
  links: number | null;
  communities: GraphifyCommunity[] | null;
  built_at_commit: string | null;
  head_commit: string | null;
  staleness: "fresh" | "stale" | "unknown";
}

/** AI-generated commit message (Rust GeneratedCommitMessage). */
export interface GeneratedCommitMessage {
  summary: string;
  description: string;
}

export interface GraphifyViewPayload {
  html: string | null;
  html_too_large: boolean;
}

/** Mirror of the Rust `UaStatus` command payload (terrain-dashboard/src/ua.rs). */
export interface UaStatus {
  data_dir: "" | "ua" | "legacy" | "terrain";
  data_dir_path: string;
  graph_present: boolean;
  meta_present: boolean;
  fingerprints_present: boolean;
  nodes: number | null;
  links: number | null;
  analyzed_at: string | null;
  built_at_commit: string | null;
  head_commit: string | null;
  staleness: "fresh" | "stale" | "unknown";
  plugin_available: boolean;
  node_available: boolean;
  node_major: number | null;
  viewer_available: boolean;
  viewer_path: string;
  install_steps_hint: string;
}

/** Mirror of the Rust `UaViewerInfo` command payload (src-tauri commands/ua.rs). */
export interface UaViewerInfo {
  running: boolean;
  url: string;
  port: number;
  message: string;
}

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
