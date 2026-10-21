<script setup lang="ts">
import { computed } from "vue";
import { parseInline } from "../../utils/guide/inline";

/** Guide text with its **bold** and `code` runs, rendered without HTML injection. */
const props = defineProps<{ text: string }>();
const parts = computed(() => parseInline(props.text));
</script>

<template>
  <template v-for="(part, index) in parts" :key="index">
    <strong v-if="part.kind === 'strong'" class="guide-strong">{{ part.text }}</strong>
    <code v-else-if="part.kind === 'code'" class="guide-code">{{ part.text }}</code>
    <template v-else>{{ part.text }}</template>
  </template>
</template>

<style scoped>
.guide-strong {
  color: var(--color-ink);
  font-weight: 600;
}

.guide-code {
  border-radius: 4px;
  background: var(--color-surface-raised);
  padding: 1px 5px;
  color: var(--color-ink);
  font-family: "Cascadia Mono", Consolas, "SF Mono", Menlo, monospace;
  font-size: 0.9em;
}
</style>
