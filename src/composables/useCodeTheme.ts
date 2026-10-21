import { computed, type Ref } from "vue";
import { usePersistentRef } from "./usePersistentRef";
import { useTheme } from "./useTheme";
import { AUTO_EDITOR_THEME, resolveEditorTheme } from "../utils/editorThemes";

// One shared choice for the settings page and the code viewer.
let choice: Ref<string> | null = null;

/** The code viewer's color theme: a chosen one, or "auto" to follow the app's dark / light theme. */
export function useCodeTheme() {
  choice ??= usePersistentRef<string>("code-theme", AUTO_EDITOR_THEME);
  const { theme } = useTheme();
  const shared = choice;
  /** The theme actually shown. */
  const activeTheme = computed(() => resolveEditorTheme(shared.value, theme.value));
  return { choice: shared, activeTheme };
}
