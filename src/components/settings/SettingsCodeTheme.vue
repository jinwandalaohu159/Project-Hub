<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "../../composables/useI18n";
import { useCodeTheme } from "../../composables/useCodeTheme";
import { AUTO_EDITOR_THEME, EDITOR_THEMES, findEditorTheme, type EditorTheme } from "../../utils/editorThemes";
import SelectMenu, { type SelectOption } from "../SelectMenu.vue";
import CodeThemeLabel from "./CodeThemeLabel.vue";
import CodeThemePreview from "./CodeThemePreview.vue";
import HelpTip from "./HelpTip.vue";

const { t } = useI18n();
const { choice, activeTheme } = useCodeTheme();

// "Follow the app" first, then every theme in the order they are listed (dark, then light).
const options = computed<SelectOption<string>[]>(() => [
  { value: AUTO_EDITOR_THEME, label: t("codeThemeAuto") },
  ...EDITOR_THEMES.map((theme) => ({ value: theme.id, label: theme.name })),
]);

/** The theme an option shows: "auto" stands for whatever the app's theme picks right now. */
function themeOf(value: string): EditorTheme {
  return value === AUTO_EDITOR_THEME ? activeTheme.value : findEditorTheme(value) ?? activeTheme.value;
}
</script>

<template>
  <div class="setting-group">
    <label class="setting-label">{{ t('codeTheme') }}<HelpTip :text="t('codeThemeDescription')" :size="12" /></label>
    <div class="setting-card space-y-3">
      <SelectMenu v-model="choice" :options="options">
        <template #selected="{ option }">
          <CodeThemeLabel :theme="themeOf(option.value)" :label="option.label" />
        </template>
        <template #option="{ option }">
          <CodeThemeLabel
            :theme="themeOf(option.value)"
            :label="option.label"
            :show-kind="option.value !== AUTO_EDITOR_THEME"
          />
        </template>
      </SelectMenu>
      <CodeThemePreview :palette="activeTheme.palette" />
    </div>
  </div>
</template>
