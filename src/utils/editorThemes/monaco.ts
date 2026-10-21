import type * as Monaco from "monaco-editor";
import type { EditorTheme, TokenRole } from "./types";
import { EDITOR_THEMES } from "./palettes";

/** Turning palettes into Monaco themes: which tokens each kind of code colors, and derived colors. */

const TOKEN_SCOPES: Record<TokenRole, string[]> = {
  comment: ["comment"],
  keyword: ["keyword"],
  string: ["string"],
  escape: ["string.escape"],
  number: ["number"],
  regexp: ["regexp"],
  type: ["type"],
  function: ["function"],
  variable: ["variable"],
  identifier: ["identifier"],
  constant: ["constant"],
  delimiter: ["delimiter"],
  operator: ["operator"],
  tag: ["tag"],
  attributeName: ["attribute.name"],
  attributeValue: ["attribute.value"],
  key: ["string.key.json"],
};

/** Adds an alpha channel to a #rrggbb color; colors that already have one are kept. */
function withAlpha(color: string, alpha: string) {
  return color.length === 7 ? `${color}${alpha}` : color;
}

export function toMonacoTheme(theme: EditorTheme): Monaco.editor.IStandaloneThemeData {
  const { palette } = theme;
  const rules: Monaco.editor.ITokenThemeRule[] = [];
  for (const [role, color] of Object.entries(palette.tokens) as [TokenRole, string][]) {
    const fontStyle = role === "comment" && palette.italicComments ? "italic" : undefined;
    for (const token of TOKEN_SCOPES[role]) {
      rules.push({ token, foreground: color.replace("#", ""), ...(fontStyle ? { fontStyle } : {}) });
    }
  }
  return {
    base: theme.kind === "light" ? "vs" : "vs-dark",
    inherit: true,
    rules,
    colors: {
      "editor.background": palette.background,
      "editor.foreground": palette.foreground,
      "editor.lineHighlightBackground": palette.lineHighlight,
      // No outline around the current line: the highlight alone marks it.
      "editor.lineHighlightBorder": palette.lineHighlight,
      "editor.selectionBackground": palette.selection,
      "editor.inactiveSelectionBackground": withAlpha(palette.selection, "80"),
      "editor.selectionHighlightBackground": withAlpha(palette.selection, "60"),
      "editorCursor.foreground": palette.cursor,
      "editorLineNumber.foreground": palette.lineNumber,
      "editorLineNumber.activeForeground": palette.lineNumberActive,
      "editorGutter.background": palette.background,
      "editorGutter.foldingControlForeground": palette.lineNumber,
      "editorIndentGuide.background": palette.indentGuide,
      "editorIndentGuide.activeBackground": palette.indentGuideActive,
      "editorBracketMatch.background": withAlpha(palette.selection, "80"),
      "editorBracketMatch.border": withAlpha(palette.lineNumber, "80"),
      "editorOverviewRuler.border": palette.indentGuide,
      "scrollbarSlider.background": withAlpha(palette.lineNumber, "30"),
      "scrollbarSlider.hoverBackground": withAlpha(palette.lineNumber, "50"),
      "scrollbarSlider.activeBackground": withAlpha(palette.lineNumber, "70"),
      ...palette.colors,
    },
  };
}

/** Registers every theme with Monaco; switching is then just `monaco.editor.setTheme(id)`. */
export function defineEditorThemes(monaco: typeof Monaco) {
  for (const theme of EDITOR_THEMES) {
    monaco.editor.defineTheme(theme.id, toMonacoTheme(theme));
  }
}
