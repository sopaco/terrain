import { describe, expect, it } from "bun:test";
import { parseAskSlashCommand } from "./askSlashCommands";

describe("parseAskSlashCommand", () => {
  it("recognizes /clear", () => {
    expect(parseAskSlashCommand("/clear")).toEqual({ type: "clear" });
    expect(parseAskSlashCommand("  /clear  ")).toEqual({ type: "clear" });
    expect(parseAskSlashCommand("/clear extra")).toEqual({ type: "clear" });
  });

  it("returns null for normal questions", () => {
    expect(parseAskSlashCommand("how does auth work?")).toBeNull();
    expect(parseAskSlashCommand("/unknown")).toBeNull();
  });
});
