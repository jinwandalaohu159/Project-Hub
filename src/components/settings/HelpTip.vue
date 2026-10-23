<script setup lang="ts">
import { CircleHelp } from "lucide-vue-next";
import Tooltip from "../Tooltip.vue";

/** A question mark after a heading or setting name; hovering it shows what the setting does. */
withDefaults(defineProps<{ text: string; size?: number }>(), { size: 13 });
</script>

<template>
  <span class="help-tip">
    <Tooltip :text="text" wrap>
      <span class="help-tip-icon" role="img" :aria-label="text"><CircleHelp :size="size" /></span>
    </Tooltip>
  </span>
</template>

<style scoped>
/* Sits in a line of text: a small gap after it, centred on the letters. */
.help-tip {
  display: inline-flex;
  margin-left: 4px;
  vertical-align: -2px;
}

.help-tip-icon {
  display: inline-flex;
  color: var(--color-muted, #8a827a);
  cursor: help;
  transition: color 0.15s ease;
}

.help-tip-icon:hover {
  color: var(--color-ink, #e8e0d8);
}
</style>
