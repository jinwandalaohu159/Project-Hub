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

export function editorThemeName(theme: string) {
  return theme === "light" ? "warm-light" : "warm-dark";
}

/** The editor themes matching the app's warm dark and light palettes. */
export function defineEditorThemes(monaco: typeof Monaco) {
  monaco.editor.defineTheme("warm-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "comment", foreground: "6e6961", fontStyle: "italic" },
      { token: "keyword", foreground: "cc785c" },
      { token: "string", foreground: "7eb87a" },
      { token: "number", foreground: "d4a856" },
      { token: "type", foreground: "6a9fd4" },
      { token: "function", foreground: "e8e0d8" },
      { token: "variable", foreground: "e8e0d8" },
      { token: "delimiter", foreground: "8a827a" },
      { token: "tag", foreground: "cc785c" },
      { token: "attribute.name", foreground: "d4a856" },
      { token: "attribute.value", foreground: "7eb87a" },
    ],
    colors: {
      "editor.background": "#1a1614",
      "editor.foreground": "#e8e0d8",
      "editor.lineHighlightBackground": "#2c2825",
      "editor.selectionBackground": "#35313080",
      "editorCursor.foreground": "#cc785c",
      "editorLineNumber.foreground": "#6e6961",
      "editorLineNumber.activeForeground": "#8a827a",
      "editor.inactiveSelectionBackground": "#2c282550",
      "editorGutter.background": "#1a1614",
      "editorGutter.foldingControlForeground": "#8a827a",
      "editorIndentGuide.background": "#2e2a27",
      "editorIndentGuide.activeBackground": "#3a3632",
      "editor.selectionHighlightBackground": "#35313040",
      "editorBracketMatch.background": "#35313060",
      "editorBracketMatch.border": "#8a827a50",
      "editorOverviewRuler.border": "#2e2a27",
      "scrollbarSlider.background": "#8a827a30",
      "scrollbarSlider.hoverBackground": "#8a827a50",
      "scrollbarSlider.activeBackground": "#8a827a70",
    },
  });

  monaco.editor.defineTheme("warm-light", {
    base: "vs",
    inherit: true,
    rules: [
      { token: "comment", foreground: "9a8a7a", fontStyle: "italic" },
      { token: "keyword", foreground: "b86a50" },
      { token: "string", foreground: "5f8f5a" },
      { token: "number", foreground: "a97925" },
      { token: "type", foreground: "5f86b0" },
      { token: "function", foreground: "2f261f" },
      { token: "variable", foreground: "2f261f" },
      { token: "delimiter", foreground: "75695d" },
      { token: "tag", foreground: "b86a50" },
      { token: "attribute.name", foreground: "a97925" },
      { token: "attribute.value", foreground: "5f8f5a" },
    ],
    colors: {
      "editor.background": "#fffaf2",
      "editor.foreground": "#2f261f",
      "editor.lineHighlightBackground": "#efe6da",
      "editor.selectionBackground": "#e7dacb",
      "editorCursor.foreground": "#cc785c",
      "editorLineNumber.foreground": "#9a8a7a",
      "editorLineNumber.activeForeground": "#5f5349",
      "editor.inactiveSelectionBackground": "#efe6da80",
      "editorGutter.background": "#fffaf2",
      "editorGutter.foldingControlForeground": "#75695d",
      "editorIndentGuide.background": "#eadfd3",
      "editorIndentGuide.activeBackground": "#d8cab9",
      "editor.selectionHighlightBackground": "#e7dacb80",
      "editorBracketMatch.background": "#e7dacb80",
      "editorBracketMatch.border": "#75695d50",
      "editorOverviewRuler.border": "#eadfd3",
      "scrollbarSlider.background": "#75695d28",
      "scrollbarSlider.hoverBackground": "#75695d44",
      "scrollbarSlider.activeBackground": "#75695d66",
    },
  });
}
