<script setup lang="ts">
import { computed } from "vue";
import { useI18n, type TranslationKey } from "../composables/useI18n";
import type { FileStatus } from "../types";

const props = defineProps<{ status: FileStatus }>();

const { t } = useI18n();
const label = computed(() => t(`status${props.status}` as TranslationKey));
</script>

<template>
  <span class="status-chip" :class="`status-${status.toLowerCase()}`">{{ label }}</span>
</template>
