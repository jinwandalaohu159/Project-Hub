<script setup lang="ts">
import type { EditorPalette } from "../../utils/editorThemes";

/** A tiny picture of a code theme: its background with three "lines of code" in its colors. */
const props = defineProps<{ palette: EditorPalette }>();

const lines = [
  { role: "keyword", width: 7 },
  { role: "string", width: 11 },
  { role: "type", width: 5 },
] as const;

function color(role: (typeof lines)[number]["role"]) {
  return props.palette.tokens[role] ?? props.palette.foreground;
}
</script>

<template>
  <span class="swatch" :style="{ background: palette.background }" aria-hidden="true">
    <span
      v-for="(line, index) in lines"
      :key="line.role"
      class="swatch-line"
      :style="{ width: `${line.width}px`, marginLeft: index === 1 ? '3px' : '0', background: color(line.role) }"
    />
  </span>
</template>

<style scoped>
.swatch {
  display: inline-flex;
  flex: none;
  flex-direction: column;
  justify-content: center;
  gap: 2px;
  width: 22px;
  height: 16px;
  border: 1px solid color-mix(in srgb, var(--color-ink) 18%, transparent);
  border-radius: 4px;
  padding: 0 3px;
}

.swatch-line {
  display: block;
  height: 2px;
  border-radius: 1px;
}
</style>
