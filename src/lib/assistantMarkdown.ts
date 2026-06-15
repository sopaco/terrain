import type { AssistantStep, ChatMessage } from "./types";

function textFromSteps(steps: AssistantStep[]): string {
  return steps
    .filter((step): step is Extract<AssistantStep, { kind: "text" }> => step.kind === "text")
    .map((step) => step.content.trim())
    .filter(Boolean)
    .join("\n\n");
}

/** Raw markdown body for an assistant reply (excludes tool traces). */
export function assistantMessageMarkdown(msg: ChatMessage): string {
  if (msg.role !== "assistant") return msg.content;
  if (msg.steps?.length) {
    const fromSteps = textFromSteps(msg.steps);
    if (fromSteps) return fromSteps;
  }
  return msg.content.trim();
}

export function assistantStepsMarkdown(steps: AssistantStep[]): string {
  return textFromSteps(steps);
}
