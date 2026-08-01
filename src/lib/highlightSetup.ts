import hljs from "highlight.js/lib/core";
import bash from "highlight.js/lib/languages/bash";
import c from "highlight.js/lib/languages/c";
import cpp from "highlight.js/lib/languages/cpp";
import csharp from "highlight.js/lib/languages/csharp";
import css from "highlight.js/lib/languages/css";
import dockerfile from "highlight.js/lib/languages/dockerfile";
import go from "highlight.js/lib/languages/go";
import ini from "highlight.js/lib/languages/ini";
import java from "highlight.js/lib/languages/java";
import javascript from "highlight.js/lib/languages/javascript";
import json from "highlight.js/lib/languages/json";
import kotlin from "highlight.js/lib/languages/kotlin";
import less from "highlight.js/lib/languages/less";
import lua from "highlight.js/lib/languages/lua";
import makefile from "highlight.js/lib/languages/makefile";
import markdown from "highlight.js/lib/languages/markdown";
import php from "highlight.js/lib/languages/php";
import python from "highlight.js/lib/languages/python";
import ruby from "highlight.js/lib/languages/ruby";
import rust from "highlight.js/lib/languages/rust";
import scala from "highlight.js/lib/languages/scala";
import scss from "highlight.js/lib/languages/scss";
import sql from "highlight.js/lib/languages/sql";
import swift from "highlight.js/lib/languages/swift";
import typescript from "highlight.js/lib/languages/typescript";
import xml from "highlight.js/lib/languages/xml";
import yaml from "highlight.js/lib/languages/yaml";

import { languageForPath } from "./sourceLanguage";

let ready = false;

export function getHighlighter(): typeof hljs {
  if (ready) return hljs;

  const languages: Array<[string, Parameters<typeof hljs.registerLanguage>[1]]> = [
    ["bash", bash],
    ["c", c],
    ["cpp", cpp],
    ["csharp", csharp],
    ["css", css],
    ["dockerfile", dockerfile],
    ["go", go],
    ["ini", ini],
    ["java", java],
    ["javascript", javascript],
    ["json", json],
    ["kotlin", kotlin],
    ["less", less],
    ["lua", lua],
    ["makefile", makefile],
    ["markdown", markdown],
    ["php", php],
    ["python", python],
    ["ruby", ruby],
    ["rust", rust],
    ["scala", scala],
    ["scss", scss],
    ["sql", sql],
    ["swift", swift],
    ["typescript", typescript],
    ["xml", xml],
    ["yaml", yaml],
  ];

  for (const [name, mod] of languages) {
    hljs.registerLanguage(name, mod);
  }

  ready = true;
  return hljs;
}

/**
 * Highlight a markdown fenced code block. Returns escaped-but-unhighlighted
 * HTML when the fence carries no language or an unregistered one — guessing
 * with `highlightAuto` on the short snippets typical of docs mislabels more
 * often than it helps.
 */
export function highlightFencedCode(code: string, lang?: string | null): string {
  const hljs = getHighlighter();
  const normalized = lang?.trim().toLowerCase();
  if (!normalized || !hljs.getLanguage(normalized)) return escapeHtml(code);

  try {
    return hljs.highlight(code, { language: normalized }).value;
  } catch {
    return escapeHtml(code);
  }
}

export interface HighlightedLine {
  number: number;
  html: string;
}

export function highlightSourceLines(
  code: string,
  filePath: string,
  startLine = 1,
): HighlightedLine[] {
  const hljs = getHighlighter();
  const lang = languageForPath(filePath);
  const lineStart = Math.max(1, startLine);

  let highlighted: string;
  try {
    if (lang && hljs.getLanguage(lang)) {
      highlighted = hljs.highlight(code, { language: lang }).value;
    } else {
      highlighted = hljs.highlightAuto(code).value;
    }
  } catch {
    highlighted = escapeHtml(code);
  }

  if (!code) {
    return [{ number: lineStart, html: " " }];
  }

  return highlighted.split("\n").map((lineHtml, index) => ({
    number: lineStart + index,
    html: lineHtml || " ",
  }));
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}
