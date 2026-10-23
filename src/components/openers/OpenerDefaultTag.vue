<script setup lang="ts">
import { useI18n } from "../../composables/useI18n";

/** Marks the program folders open with by default. */
const { t } = useI18n();
</script>

<template>
  <span class="opener-default">{{ t('openWithDefault') }}</span>
</template>

<style scoped>
.opener-default {
  flex: none;
  border-radius: 4px;
  background: color-mix(in srgb, var(--color-primary) 16%, transparent);
  padding: 0 6px;
  color: var(--color-primary);
  font-size: 11px;
  line-height: 18px;
}
</style>
