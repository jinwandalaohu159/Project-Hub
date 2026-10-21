import type * as Monaco from "monaco-editor";

/** Monaco is large: it is loaded on first use, once. */
let monacoPromise: Promise<typeof Monaco> | null = null;

export function loadMonaco(): Promise<typeof Monaco> {
  monacoPromise ??= import("monaco-editor");
  return monacoPromise;
}

const LANGUAGES: Record<string, string> = {
  ts: "typescript", tsx: "typescript", js: "javascript", jsx: "javascript",
  vue: "html", rs: "rust", py: "python", json: "json", md: "markdown",
  css: "css", html: "html", toml: "ini", yaml: "yaml", yml: "yaml",
  xml: "xml", sql: "sql", sh: "shell", bash: "shell",
};

/** Monaco language for a file, by extension. */
export function languageForPath(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return LANGUAGES[ext] ?? "plaintext";
}
