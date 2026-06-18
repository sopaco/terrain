/**
 * Public type surface for the MindMesh desktop UI.
 *
 * - IPC types: auto-generated from Rust (`bun run gen:types`)
 * - UI-only types: `types.client.ts`
 */
export * from "./generated";
export {
  type AppTab,
  type AskExecution,
  type AskKnowledgeReplyUi,
  type AssistantStep,
  type ChatMessage,
  type KnowledgeDoc,
  type LegacyAgentExecution,
  type SourceSlice,
  normalizeAskReply,
} from "./types.client";

// Ergonomic aliases for generated names
export type { ChatToolCallStatus as ToolCallStatus } from "./generated";
