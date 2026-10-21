import { describe, it, expect, vi } from "vitest";
import {
  AUTO_EDITOR_THEME,
  EDITOR_THEMES,
  defineEditorThemes,
  resolveEditorTheme,
  toMonacoTheme,
} from "../utils/editorThemes";

const HEX = /^#[0-9a-f]{6}([0-9a-f]{2})?$/i;

describe("editor themes", () => {
  it("are valid Monaco themes with unique names", () => {
    const ids = EDITOR_THEMES.map((theme) => theme.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const theme of EDITOR_THEMES) {
      expect(theme.id).toMatch(/^[a-z0-9-]+$/);
      const data = toMonacoTheme(theme);
      expect(data.base).toBe(theme.kind === "light" ? "vs" : "vs-dark");
      for (const color of Object.values(data.colors)) expect(color).toMatch(HEX);
      for (const rule of data.rules) expect(`#${rule.foreground}`).toMatch(HEX);
    }
  });

  it("are listed dark first, then light, as the picker shows them", () => {
    const kinds = EDITOR_THEMES.map((theme) => theme.kind);
    expect(kinds).toEqual([...kinds].sort((x, y) => Number(x === "light") - Number(y === "light")));
  });

  it("follows the app theme for auto and for a theme that no longer exists", () => {
    expect(resolveEditorTheme(AUTO_EDITOR_THEME, "dark").id).toBe("warm-dark");
    expect(resolveEditorTheme(AUTO_EDITOR_THEME, "light").id).toBe("warm-light");
    expect(resolveEditorTheme("removed-theme", "light").id).toBe("warm-light");
    expect(resolveEditorTheme("monokai", "light").id).toBe("monokai");
  });

  it("keeps the original warm theme exactly", () => {
    const data = toMonacoTheme(EDITOR_THEMES.find((theme) => theme.id === "warm-dark")!);
    expect(data.rules).toContainEqual({ token: "comment", foreground: "6e6961", fontStyle: "italic" });
    expect(data.rules).toContainEqual({ token: "keyword", foreground: "cc785c" });
    expect(data.rules.some((rule) => rule.token === "identifier")).toBe(false);
    expect(data.colors["editor.background"]).toBe("#1a1614");
    expect(data.colors["editor.lineHighlightBorder"]).toBe("#282828");
    expect(data.colors["editor.selectionHighlightBackground"]).toBe("#35313040");
  });

  it("registers every theme with Monaco", () => {
    const defineTheme = vi.fn();
    defineEditorThemes({ editor: { defineTheme } } as never);
    expect(defineTheme).toHaveBeenCalledTimes(EDITOR_THEMES.length);
  });
});
