<script setup lang="ts">
import { Eye, PenLine } from "lucide-vue-next";
import { useI18n } from "../../composables/useI18n";

/** Whether a tool only reads or also changes the project. */
defineProps<{ readOnly: boolean }>();
const { t } = useI18n();
</script>

<template>
  <span class="tool-badge" :class="{ writes: !readOnly }">
    <Eye v-if="readOnly" :size="11" /><PenLine v-else :size="11" />
    {{ readOnly ? t('mcpToolReads') : t('mcpToolWrites') }}
  </span>
</template>

<style scoped>
.tool-badge {
  display: inline-flex;
  flex: none;
  align-items: center;
  gap: 4px;
  border-radius: 999px;
  background: var(--color-surface-raised);
  padding: 1px 8px;
  color: var(--color-muted);
  font-size: 11px;
}

.tool-badge.writes {
  background: color-mix(in srgb, var(--color-primary) 16%, transparent);
  color: var(--color-primary);
}
</style>
