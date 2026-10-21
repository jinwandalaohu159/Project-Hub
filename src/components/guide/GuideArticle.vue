<script setup lang="ts">
import { ArrowRight } from "lucide-vue-next";
import GuideBlock from "./GuideBlock.vue";
import { useI18n } from "../../composables/useI18n";
import type { GuideAction, GuideSection, GuideSectionId } from "../../utils/guide";

/** Every chapter of the guide, one after another, each ending with a way on to the next. */
defineProps<{ sections: GuideSection[] }>();
const emit = defineEmits<{ select: [id: GuideSectionId]; action: [action: GuideAction] }>();

const { t } = useI18n();
</script>

<template>
  <article class="guide-article">
    <section v-for="(section, index) in sections" :key="section.id" class="guide-section" :data-section="section.id">
      <div class="guide-section-index">{{ String(index + 1).padStart(2, "0") }}</div>
      <h2 class="guide-section-title">{{ section.title }}</h2>
      <p class="guide-section-summary">{{ section.summary }}</p>
      <GuideBlock v-for="(block, blockIndex) in section.blocks" :key="blockIndex" :block="block" @action="emit('action', $event)" />
      <button
        v-if="sections[index + 1]"
        type="button"
        class="guide-next"
        @click="emit('select', sections[index + 1].id)"
      >
        <span class="guide-next-label">{{ t('guideNextChapter') }}</span>
        <span class="guide-next-title">{{ sections[index + 1].title }}</span>
        <ArrowRight :size="15" class="guide-next-arrow" />
      </button>
    </section>
  </article>
</template>

<style scoped>
.guide-article {
  max-width: 680px;
  padding-bottom: 32px;
}

.guide-section {
  padding-top: 26px;
}

.guide-section + .guide-section {
  margin-top: 18px;
  border-top: 1px solid var(--color-border-subtle);
}

.guide-section-index {
  color: var(--color-primary);
  font-size: 12px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  letter-spacing: 0.08em;
}

.guide-section-title {
  margin-top: 4px;
  color: var(--color-ink);
  font-size: var(--font-size-h1);
  font-weight: 700;
  line-height: 1.3;
}

.guide-section-summary {
  margin-top: 4px;
  color: var(--color-muted);
  font-size: 13px;
}

.guide-next {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  margin-top: 22px;
  border: 1px solid var(--color-border-subtle);
  border-radius: 10px;
  padding: 10px 14px;
  text-align: left;
  transition: border-color var(--dur-fast) ease, background-color var(--dur-fast) ease;
}

.guide-next:hover {
  border-color: color-mix(in srgb, var(--color-primary) 50%, var(--color-border));
  background: color-mix(in srgb, var(--color-primary) 6%, transparent);
}

.guide-next-label {
  flex: none;
  color: var(--color-muted);
  font-size: 12px;
}

.guide-next-title {
  min-width: 0;
  flex: 1;
  overflow: hidden;
  color: var(--color-ink);
  font-size: 13.5px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.guide-next-arrow {
  flex: none;
  color: var(--color-primary);
  transition: transform var(--dur-fast) ease;
}

.guide-next:hover .guide-next-arrow {
  transform: translateX(3px);
}
</style>
