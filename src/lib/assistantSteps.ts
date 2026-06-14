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

export function startNewTextStep(steps: AssistantStep[]): AssistantStep[] {
  const last = steps[steps.length - 1];
  if (last?.kind === "text" && !last.content.trim()) {
    return steps;
  }
  return [...steps, { kind: "text", content: "" }];
}

export function syncStepTools(steps: AssistantStep[], calls: ToolCallRecord[]): AssistantStep[] {
  if (calls.length === 0) return steps;
  const next = [...steps];
  const last = next[next.length - 1];
  const running = calls.some((c) => c.status === "running");

  if (last?.kind === "tools") {
    const batchDone = last.toolCalls.every((c) => c.status !== "running");
    if (batchDone && running) {
      next.push({ kind: "tools", toolCalls: calls });
    } else {
      next[next.length - 1] = { kind: "tools", toolCalls: calls };
    }
  } else {
    next.push({ kind: "tools", toolCalls: calls });
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
