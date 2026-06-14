import { describe, expect, test } from "bun:test";
import { syncStepTools } from "./assistantSteps";
import type { ToolCallRecord } from "./types";

function call(id: string, status: ToolCallRecord["status"] = "ok"): ToolCallRecord {
  return {
    id,
    name: "grep_agent_pack",
    arguments: { pattern: id },
    status,
    started_at: Date.now(),
  };
}

describe("syncStepTools", () => {
  test("tracks only the current batch while tools are running", () => {
    let steps = syncStepTools([], [call("a", "running")]);
    expect(steps).toHaveLength(1);
    expect(steps[0].kind === "tools" && steps[0].toolCalls.map((c) => c.id)).toEqual(["a"]);

    steps = syncStepTools(steps, [call("a", "ok")]);
    expect(steps).toHaveLength(1);
    expect(steps[0].kind === "tools" && steps[0].toolCalls.map((c) => c.id)).toEqual(["a"]);
  });

  test("starts a new batch without repeating earlier tool calls", () => {
    let steps = syncStepTools([], [call("a", "running")]);
    steps = syncStepTools(steps, [call("a", "ok")]);
    steps = syncStepTools(steps, [call("a", "ok"), call("b", "running")]);

    expect(steps).toHaveLength(2);
    expect(steps[0].kind === "tools" && steps[0].toolCalls.map((c) => c.id)).toEqual(["a"]);
    expect(steps[1].kind === "tools" && steps[1].toolCalls.map((c) => c.id)).toEqual(["b"]);
  });

  test("updates the active batch without pulling in prior batches", () => {
    let steps = syncStepTools([], [call("a", "running")]);
    steps = syncStepTools(steps, [call("a", "ok")]);
    steps = syncStepTools(steps, [call("a", "ok"), call("b", "running")]);
    steps = syncStepTools(steps, [call("a", "ok"), call("b", "ok")]);
    steps = syncStepTools(steps, [
      call("a", "ok"),
      call("b", "ok"),
      call("c", "running"),
    ]);

    expect(steps).toHaveLength(3);
    expect(steps[0].kind === "tools" && steps[0].toolCalls.map((c) => c.id)).toEqual(["a"]);
    expect(steps[1].kind === "tools" && steps[1].toolCalls.map((c) => c.id)).toEqual(["b"]);
    expect(steps[2].kind === "tools" && steps[2].toolCalls.map((c) => c.id)).toEqual(["c"]);
  });

  test("keeps parallel tools in the same batch", () => {
    let steps = syncStepTools([], [call("a", "running"), call("b", "running")]);
    steps = syncStepTools(steps, [call("a", "ok"), call("b", "ok"), call("c", "running")]);

    expect(steps).toHaveLength(2);
    expect(steps[0].kind === "tools" && steps[0].toolCalls.map((c) => c.id)).toEqual(["a", "b"]);
    expect(steps[1].kind === "tools" && steps[1].toolCalls.map((c) => c.id)).toEqual(["c"]);
  });

  test("inserts a tools step after text without duplicating assigned calls", () => {
    const steps = syncStepTools(
      [{ kind: "text", content: "Thinking…" }, { kind: "tools", toolCalls: [call("a", "ok")] }],
      [call("a", "ok"), call("b", "running")],
    );

    expect(steps).toHaveLength(3);
    expect(steps[1].kind === "tools" && steps[1].toolCalls.map((c) => c.id)).toEqual(["a"]);
    expect(steps[2].kind === "tools" && steps[2].toolCalls.map((c) => c.id)).toEqual(["b"]);
  });
});
