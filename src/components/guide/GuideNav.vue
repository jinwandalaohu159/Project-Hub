<script setup lang="ts">
import type { GuideSection, GuideSectionId } from "../../utils/guide";

/** The guide's table of contents; follows the chapter being read. */
defineProps<{ sections: GuideSection[]; active: GuideSectionId }>();
const emit = defineEmits<{ select: [id: GuideSectionId] }>();
</script>

<template>
  <nav class="guide-nav">
    <button
      v-for="(section, index) in sections"
      :key="section.id"
      type="button"
      class="guide-nav-item"
      :class="{ active: section.id === active }"
      :title="section.summary"
      :aria-current="section.id === active ? 'true' : undefined"
      @click="emit('select', section.id)"
    >
      <span class="guide-nav-number">{{ String(index + 1).padStart(2, "0") }}</span>
      <span class="min-w-0">{{ section.title }}</span>
    </button>
  </nav>
</template>

<style scoped>
.guide-nav {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.guide-nav-item {
  position: relative;
  display: flex;
  align-items: baseline;
  gap: 9px;
  border-radius: 7px;
  padding: 7px 10px;
  color: var(--color-muted);
  font-size: 13px;
  line-height: 1.45;
  text-align: left;
  transition: background-color var(--dur-fast) ease, color var(--dur-fast) ease;
}

.guide-nav-item:hover {
  background: var(--color-surface-raised);
  color: var(--color-ink);
}

.guide-nav-item.active {
  background: color-mix(in srgb, var(--color-primary) 13%, transparent);
  color: var(--color-ink);
  font-weight: 600;
}

.guide-nav-item.active::before {
  position: absolute;
  top: 8px;
  bottom: 8px;
  left: 0;
  width: 3px;
  border-radius: 3px;
  background: var(--color-primary);
  content: "";
}

.guide-nav-number {
  flex: none;
  color: var(--color-muted);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
  font-weight: 500;
}

.guide-nav-item.active .guide-nav-number {
  color: var(--color-primary);
}
</style>
