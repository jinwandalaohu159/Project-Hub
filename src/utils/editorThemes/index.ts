import type { EditorTheme, ThemeKind } from "./types";
import { EDITOR_THEMES } from "./palettes";

/**
 * The code viewer's color themes. Each is one palette (palettes.ts), turned into a Monaco theme
 * by monaco.ts; this module picks the theme to show for the stored choice.
 */
export type { EditorPalette, EditorTheme, ThemeKind, TokenRole } from "./types";
export { EDITOR_THEMES } from "./palettes";
export { defineEditorThemes, toMonacoTheme } from "./monaco";

/** Follows the app's dark / light choice with Project Hub's own warm themes. */
export const AUTO_EDITOR_THEME = "auto";

export function findEditorTheme(id: string): EditorTheme | undefined {
  return EDITOR_THEMES.find((theme) => theme.id === id);
}

/** The theme to show for a stored choice: "auto" (or a theme that no longer exists) follows the app. */
export function resolveEditorTheme(choice: string, appTheme: ThemeKind): EditorTheme {
  const chosen = choice === AUTO_EDITOR_THEME ? undefined : findEditorTheme(choice);
  return chosen ?? findEditorTheme(appTheme === "light" ? "warm-light" : "warm-dark")!;
}
