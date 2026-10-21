<script setup lang="ts">
import { useI18n } from "../../composables/useI18n";
import type { EditorTheme } from "../../utils/editorThemes";
import CodeThemeSwatch from "./CodeThemeSwatch.vue";

/** A code theme as the picker shows it: its swatch, its name and, in the list, dark or light. */
defineProps<{
  theme: EditorTheme;
  label: string;
  showKind?: boolean;
}>();

const { t } = useI18n();
</script>

<template>
  <span class="theme-label">
    <CodeThemeSwatch :palette="theme.palette" />
    <span class="min-w-0 flex-1 truncate">{{ label }}</span>
    <span v-if="showKind" class="theme-kind">
      {{ theme.kind === 'light' ? t('lightMode') : t('darkMode') }}
    </span>
  </span>
</template>

<style scoped>
.theme-label {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 8px;
}

.theme-kind {
  flex: none;
  color: var(--color-muted);
  font-size: 11px;
}
</style>
