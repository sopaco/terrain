export type AskSlashCommand = { type: "clear" };

/** Parse a leading slash command from Ask / DeepWiki input. */
export function parseAskSlashCommand(input: string): AskSlashCommand | null {
  const trimmed = input.trim();
  if (!trimmed.startsWith("/")) return null;

  const [command] = trimmed.split(/\s+/, 1);
  if (command === "/clear") {
    return { type: "clear" };
  }
  return null;
}
