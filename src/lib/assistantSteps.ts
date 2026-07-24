import { prepareMarkdownForRender } from "./markdownSanitize";
import type { AssistantStep, ToolCallRecord } from "./types";

export function appendStepText(steps: AssistantStep[], chunk: string): AssistantStep[] {
  if (!chunk) return steps;
  const next = [...steps];
  const last = next[next.length - 1];
  if (last?.kind === "text") {
    next[next.length - 1] = { kind: "text", content: last.content + chunk };
  } else {
    next.push({ kind: "text", content: chunk });
  }
  return next;
}

export function appendStepThinking(steps: AssistantStep[], chunk: string): AssistantStep[] {
  if (!chunk) return steps;
  const next = [...steps];
  const last = next[next.length - 1];
  if (last?.kind === "thinking") {
    next[next.length - 1] = { kind: "thinking", content: last.content + chunk };
  } else {
    next.push({ kind: "thinking", content: chunk });
  }
  return next;
}

/** Last text step index, or -1 when none. */
export function lastTextStepIndex(steps: AssistantStep[]): number {
  for (let i = steps.length - 1; i >= 0; i -= 1) {
    if (steps[i].kind === "text") return i;
  }
  return -1;
}

/** Thinking steps and legacy pre-answer text steps (before the final answer text). */
export function isThinkingStep(steps: AssistantStep[], index: number): boolean {
  const step = steps[index];
  if (step.kind === "thinking") return step.content.trim().length > 0;
  if (step.kind !== "text" || !step.content.trim()) return false;
  const lastText = lastTextStepIndex(steps);
  return lastText >= 0 && index !== lastText;
}

export function isThinkingStepActive(
  steps: AssistantStep[],
  index: number,
  busy: boolean,
): boolean {
  if (!busy || !isThinkingStep(steps, index)) return false;
  const lastText = lastTextStepIndex(steps);
  if (lastText < 0) return true;
  const step = steps[lastText];
  return step.kind === "text" && !step.content.trim();
}

export function startNewTextStep(steps: AssistantStep[]): AssistantStep[] {
  const last = steps[steps.length - 1];
  if (last?.kind === "text" && !last.content.trim()) {
    return steps;
  }
  return [...steps, { kind: "text", content: "" }];
}

function assignedToolCount(steps: AssistantStep[]): number {
  return steps.reduce(
    (count, step) => count + (step.kind === "tools" ? step.toolCalls.length : 0),
    0,
  );
}

/** Sync streaming tool events into per-batch steps (backend emits cumulative calls). */
export function syncStepTools(steps: AssistantStep[], calls: ToolCallRecord[]): AssistantStep[] {
  if (calls.length === 0) return steps;

  const next = [...steps];
  const alreadyAssigned = assignedToolCount(next);
  const pending = calls.slice(alreadyAssigned);
  const last = next[next.length - 1];

  if (last?.kind === "tools") {
    const batchStart = alreadyAssigned - last.toolCalls.length;
    const currentBatch = calls.slice(batchStart, alreadyAssigned);

    if (pending.length > 0) {
      next[next.length - 1] = { kind: "tools", toolCalls: currentBatch };
      next.push({ kind: "tools", toolCalls: pending });
    } else {
      next[next.length - 1] = { kind: "tools", toolCalls: currentBatch };
    }
  } else if (pending.length > 0) {
    next.push({ kind: "tools", toolCalls: pending });
  }

  return next;
}

export function finalizeAssistantSteps(
  steps: AssistantStep[],
  finalAnswer: string,
  allToolCalls: ToolCallRecord[],
): AssistantStep[] {
  let next = steps.length > 0 ? [...steps] : [];

  if (next.length === 0 && allToolCalls.length > 0) {
    next.push({ kind: "tools", toolCalls: allToolCalls });
  }

  const answer = prepareMarkdownForRender(finalAnswer);
  if (!answer) return next;

  const last = next[next.length - 1];
  if (last?.kind === "text") {
    const streamed = prepareMarkdownForRender(last.content);
    if (!streamed) {
      next[next.length - 1] = { kind: "text", content: answer };
    } else if (streamed === answer || streamed.includes(answer)) {
      next[next.length - 1] = { kind: "text", content: streamed };
    } else if (answer.includes(streamed)) {
      next[next.length - 1] = { kind: "text", content: answer };
    } else {
      next.push({ kind: "text", content: answer });
    }
  } else {
    next.push({ kind: "text", content: answer });
  }
  return next;
}

export function finalAnswerFromSteps(steps: AssistantStep[], fallback: string): string {
  for (let i = steps.length - 1; i >= 0; i--) {
    const step = steps[i];
    if (step.kind === "text" && step.content.trim()) {
      return step.content.trim();
    }
  }
  return fallback.trim();
}
