<script setup lang="ts">
import { ArrowRight, ChevronRight, Lightbulb, TriangleAlert } from "lucide-vue-next";
import GuideText from "./GuideText.vue";
import type { GuideAction, GuideBlock } from "../../utils/guide";

/** One block of a guide chapter: a paragraph, steps, a tip, shortcuts… */
defineProps<{ block: GuideBlock }>();
const emit = defineEmits<{ action: [action: GuideAction] }>();
</script>

<template>
  <p v-if="block.type === 'paragraph'" class="guide-paragraph"><GuideText :text="block.text" /></p>

  <h3 v-else-if="block.type === 'heading'" class="guide-heading">{{ block.text }}</h3>

  <ol v-else-if="block.type === 'steps'" class="guide-steps">
    <li v-for="(item, index) in block.items" :key="index" class="guide-step">
      <span class="guide-step-number">{{ index + 1 }}</span>
      <span class="min-w-0"><GuideText :text="item" /></span>
    </li>
  </ol>

  <ul v-else-if="block.type === 'list'" class="guide-list">
    <li v-for="(item, index) in block.items" :key="index"><GuideText :text="item" /></li>
  </ul>

  <dl v-else-if="block.type === 'terms'" class="guide-terms">
    <template v-for="item in block.items" :key="item.term">
      <dt class="guide-term">{{ item.term }}</dt>
      <dd class="guide-term-text"><GuideText :text="item.text" /></dd>
    </template>
  </dl>

  <div v-else-if="block.type === 'tip' || block.type === 'warning'" class="guide-callout" :class="`guide-callout-${block.type}`">
    <Lightbulb v-if="block.type === 'tip'" :size="15" class="guide-callout-icon" />
    <TriangleAlert v-else :size="15" class="guide-callout-icon" />
    <p class="min-w-0"><GuideText :text="block.text" /></p>
  </div>

  <div v-else-if="block.type === 'keys'" class="guide-keys">
    <div v-for="item in block.items" :key="item.text" class="guide-key-row">
      <span class="guide-key-combo">
        <template v-for="(key, index) in item.keys" :key="key">
          <span v-if="index > 0" class="guide-key-plus">+</span><kbd class="guide-kbd">{{ key }}</kbd>
        </template>
      </span>
      <span class="text-ink">{{ item.text }}</span>
    </div>
  </div>

  <details v-else-if="block.type === 'qa'" class="guide-qa">
    <summary class="guide-question">
      <ChevronRight :size="14" class="guide-qa-chevron" />
      {{ block.question }}
    </summary>
    <p class="guide-answer"><GuideText :text="block.answer" /></p>
  </details>

  <button v-else-if="block.type === 'action'" type="button" class="btn btn-primary btn-sm guide-action" @click="emit('action', block.action)">
    {{ block.label }}<ArrowRight :size="14" />
  </button>
</template>

<style scoped>
.guide-paragraph,
.guide-list,
.guide-steps,
.guide-terms,
.guide-callout,
.guide-keys,
.guide-action {
  margin-top: 12px;
}

.guide-paragraph,
.guide-list li,
.guide-step,
.guide-term-text,
.guide-callout p,
.guide-answer {
  color: color-mix(in srgb, var(--color-ink) 82%, var(--color-muted));
  font-size: 13.5px;
  line-height: 1.75;
}

.guide-heading {
  margin-top: 22px;
  color: var(--color-ink);
  font-size: 14.5px;
  font-weight: 600;
}

.guide-steps {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.guide-step {
  display: flex;
  gap: 10px;
}

.guide-step-number {
  display: flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  margin-top: 1px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-primary) 16%, transparent);
  color: var(--color-primary);
  font-size: 12px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.guide-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding-left: 18px;
  list-style: disc;
}

.guide-list li::marker {
  color: var(--color-primary);
}

.guide-terms {
  display: grid;
  grid-template-columns: minmax(84px, max-content) 1fr;
  overflow: hidden;
  border: 1px solid var(--color-border-subtle);
  border-radius: 10px;
}

.guide-term,
.guide-term-text {
  padding: 9px 12px;
}

.guide-term {
  background: var(--color-surface-raised);
  color: var(--color-ink);
  font-size: 13px;
  font-weight: 600;
  line-height: 1.75;
}

.guide-term:not(:first-of-type),
.guide-term-text:not(:first-of-type) {
  border-top: 1px solid var(--color-border-subtle);
}

.guide-callout {
  display: flex;
  gap: 10px;
  border-radius: 10px;
  padding: 10px 12px;
}

.guide-callout-tip {
  background: color-mix(in srgb, var(--color-info) 10%, transparent);
  box-shadow: inset 3px 0 0 color-mix(in srgb, var(--color-info) 70%, transparent);
}

.guide-callout-warning {
  background: color-mix(in srgb, var(--color-warning) 11%, transparent);
  box-shadow: inset 3px 0 0 color-mix(in srgb, var(--color-warning) 75%, transparent);
}

.guide-callout-icon {
  flex: none;
  margin-top: 4px;
}

.guide-callout-tip .guide-callout-icon {
  color: var(--color-info);
}

.guide-callout-warning .guide-callout-icon {
  color: var(--color-warning);
}

.guide-keys {
  overflow: hidden;
  border: 1px solid var(--color-border-subtle);
  border-radius: 10px;
}

.guide-key-row {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 8px 12px;
  font-size: 13px;
}

.guide-key-row + .guide-key-row {
  border-top: 1px solid var(--color-border-subtle);
}

.guide-key-combo {
  display: flex;
  flex: none;
  align-items: center;
  gap: 4px;
  min-width: 112px;
}

.guide-key-plus {
  color: var(--color-muted);
  font-size: 11px;
}

.guide-kbd {
  min-width: 24px;
  border: 1px solid var(--color-border);
  border-bottom-width: 2px;
  border-radius: 5px;
  background: var(--color-canvas);
  padding: 1px 7px;
  color: var(--color-ink);
  font-family: inherit;
  font-size: 11.5px;
  font-weight: 600;
  text-align: center;
}

.guide-qa {
  margin-top: 8px;
  border: 1px solid var(--color-border-subtle);
  border-radius: 10px;
  transition: border-color var(--dur-fast) ease;
}

.guide-qa[open] {
  border-color: var(--color-border);
}

.guide-question {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  color: var(--color-ink);
  cursor: pointer;
  font-size: 13.5px;
  font-weight: 600;
  list-style: none;
}

.guide-question::-webkit-details-marker {
  display: none;
}

.guide-qa-chevron {
  flex: none;
  color: var(--color-muted);
  transition: transform var(--dur-fast) ease;
}

.guide-qa[open] .guide-qa-chevron {
  transform: rotate(90deg);
}

.guide-answer {
  padding: 0 12px 11px 34px;
}

.guide-action {
  gap: 6px;
}
</style>
