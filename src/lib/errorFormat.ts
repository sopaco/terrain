const SUMMARY_MAX = 240;

export function errorToText(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  if (error == null) return "";
  try {
    return JSON.stringify(error, null, 2);
  } catch {
    return String(error);
  }
}

function tryParseJsonMessage(jsonText: string): string | null {
  try {
    return extractMessageFromParsed(JSON.parse(jsonText) as unknown);
  } catch {
    return null;
  }
}

function extractMessageFromParsed(parsed: unknown): string | null {
  if (!parsed || typeof parsed !== "object") return null;
  const record = parsed as Record<string, unknown>;
  const nestedError = record.error;
  const candidates = [
    record.message,
    typeof nestedError === "object" && nestedError
      ? (nestedError as Record<string, unknown>).message
      : undefined,
    record.detail,
    record.error_description,
    record.reason,
    typeof nestedError === "string" ? nestedError : undefined,
  ];
  for (const candidate of candidates) {
    if (typeof candidate === "string" && candidate.trim()) {
      return candidate.trim();
    }
  }
  return null;
}

/** Pull a human-readable line out of verbose API / IPC error strings. */
export function extractHumanMessage(text: string): string {
  const trimmed = text.trim();
  if (!trimmed) return trimmed;

  const jsonSuffix = trimmed.match(/:\s*(\{[\s\S]*\})\s*$/);
  if (jsonSuffix?.index != null) {
    const parsed = tryParseJsonMessage(jsonSuffix[1]);
    if (parsed) {
      const prefix = trimmed.slice(0, jsonSuffix.index).replace(/:\s*$/, "").trim();
      return prefix ? `${prefix}: ${parsed}` : parsed;
    }
  }

  if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
    const parsed = tryParseJsonMessage(trimmed);
    if (parsed) return parsed;
  }

  return trimmed;
}

function truncate(text: string, max: number): string {
  if (text.length <= max) return text;
  return `${text.slice(0, max - 1)}…`;
}

export function formatErrorDisplay(error: unknown): {
  summary: string;
  detail: string | null;
} {
  const detail = errorToText(error).trim();
  if (!detail) {
    return { summary: "Unknown error", detail: null };
  }

  const human = extractHumanMessage(detail);
  const summary = truncate(human, SUMMARY_MAX);
  const needsDetail =
    detail.length > SUMMARY_MAX || human !== detail || summary.length < human.length;

  return {
    summary,
    detail: needsDetail ? detail : null,
  };
}

export function formatErrorSummary(error: unknown): string {
  return formatErrorDisplay(error).summary;
}
