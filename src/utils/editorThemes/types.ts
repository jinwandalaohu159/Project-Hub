/** The shape of a code theme: one palette of editor colors and a color per kind of code. */

export type ThemeKind = "dark" | "light";

/** Kinds of code a palette colors; monaco.ts maps each to the Monaco tokens it covers. */
export type TokenRole =
  | "comment"
  | "keyword"
  | "string"
  | "escape"
  | "number"
  | "regexp"
  | "type"
  | "function"
  | "variable"
  | "identifier"
  | "constant"
  | "delimiter"
  | "operator"
  | "tag"
  | "attributeName"
  | "attributeValue"
  | "key";

export interface EditorPalette {
  background: string;
  foreground: string;
  lineHighlight: string;
  selection: string;
  lineNumber: string;
  lineNumberActive: string;
  cursor: string;
  indentGuide: string;
  indentGuideActive: string;
  tokens: Partial<Record<TokenRole, string>>;
  italicComments?: boolean;
  /** Exact editor colors that should not be derived from the palette. */
  colors?: Record<string, string>;
}

export interface EditorTheme {
  /** Monaco theme name; also what the setting stores. */
  id: string;
  name: string;
  kind: ThemeKind;
  palette: EditorPalette;
}
