<script setup lang="ts">
import type { Component } from "vue";
import { Loader2 } from "lucide-vue-next";
import Tooltip from "./Tooltip.vue";

/** One of the upload / download buttons: an icon, a label, and a count of records waiting. */
defineProps<{
  icon: Component;
  label: string;
  tooltip: string;
  loading: boolean;
  disabled: boolean;
  /** Shown as a badge while not loading, e.g. "↑2". */
  badge?: string | null;
}>();
defineEmits<{ click: [] }>();
</script>

<template>
  <Tooltip :text="tooltip" position="bottom">
    <button class="cloud-btn" :class="{ pending: !!badge }" :disabled="disabled" @click="$emit('click')">
      <Loader2 v-if="loading" :size="14" class="spin" />
      <component :is="icon" v-else :size="14" />
      <span>{{ label }}</span>
      <Transition name="fade">
        <span v-if="badge && !loading" class="cloud-badge">{{ badge }}</span>
      </Transition>
    </button>
  </Tooltip>
</template>

<style scoped>
.cloud-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 26px;
  border: 1px solid var(--color-border);
  border-radius: 7px;
  background: var(--color-surface);
  padding: 0 9px;
  color: var(--color-muted);
  font-size: 12px;
  white-space: nowrap;
}

.cloud-btn:hover:not(:disabled) {
  border-color: color-mix(in srgb, var(--color-primary) 55%, var(--color-border));
  color: var(--color-ink);
}

.cloud-btn.pending {
  color: var(--color-ink);
}

.cloud-btn:disabled {
  opacity: 0.6;
  cursor: default;
}

.cloud-badge {
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-primary) 22%, transparent);
  padding: 0 6px;
  color: var(--color-primary);
  font-size: 11px;
  line-height: 16px;
}
</style>
