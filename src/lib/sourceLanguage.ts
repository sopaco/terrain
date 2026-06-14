const EXT_TO_LANG: Record<string, string> = {
  rs: "rust",
  ts: "typescript",
  tsx: "typescript",
  js: "javascript",
  jsx: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  py: "python",
  go: "go",
  java: "java",
  kt: "kotlin",
  kts: "kotlin",
  swift: "swift",
  cs: "csharp",
  cpp: "cpp",
  cc: "cpp",
  cxx: "cpp",
  c: "c",
  h: "c",
  hpp: "cpp",
  md: "markdown",
  yaml: "yaml",
  yml: "yaml",
  toml: "ini",
  json: "json",
  jsonc: "json",
  html: "xml",
  htm: "xml",
  xml: "xml",
  svg: "xml",
  css: "css",
  scss: "scss",
  sass: "scss",
  less: "less",
  sql: "sql",
  sh: "bash",
  bash: "bash",
  zsh: "bash",
  fish: "bash",
  dockerfile: "dockerfile",
  rb: "ruby",
  php: "php",
  lua: "lua",
  r: "r",
  scala: "scala",
  vue: "xml",
  svelte: "xml",
};

/** Map a repository file path to a highlight.js language id. */
export function languageForPath(filePath: string): string | null {
  const base = filePath.split(/[/\\]/).pop() ?? filePath;
  const dot = base.lastIndexOf(".");
  if (dot <= 0) {
    if (base.toLowerCase() === "dockerfile") return "dockerfile";
    if (base.toLowerCase() === "makefile") return "makefile";
    return null;
  }
  const ext = base.slice(dot + 1).toLowerCase();
  return EXT_TO_LANG[ext] ?? null;
}
