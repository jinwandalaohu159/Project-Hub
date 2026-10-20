<script setup lang="ts">
import { nextTick, ref, onBeforeUnmount } from 'vue'

const props = withDefaults(defineProps<{
  text: string
  position?: 'top' | 'bottom'
}>(), {
  position: 'top',
})

const triggerRef = ref<HTMLElement | null>(null)
const tooltipRef = ref<HTMLElement | null>(null)
const visible = ref(false)
const tooltipStyle = ref<Record<string, string>>({})

const GAP = 6
const VIEWPORT_PADDING = 8

async function show() {
  visible.value = true
  await nextTick()
  updatePosition()
}

function hide() {
  visible.value = false
}

function updatePosition() {
  const el = triggerRef.value
  const tooltip = tooltipRef.value
  if (!el) return

  const rect = el.getBoundingClientRect()
  const tooltipWidth = tooltip?.offsetWidth ?? 0
  const tooltipHeight = tooltip?.offsetHeight ?? 0
  const centeredLeft = rect.left + rect.width / 2
  const minLeft = VIEWPORT_PADDING + tooltipWidth / 2
  const maxLeft = window.innerWidth - VIEWPORT_PADDING - tooltipWidth / 2
  const left = Math.min(Math.max(centeredLeft, minLeft), maxLeft)
  const topSpace = rect.top
  const bottomSpace = window.innerHeight - rect.bottom
  const shouldShowOnTop =
    props.position === 'top'
      ? topSpace >= tooltipHeight + GAP || topSpace >= bottomSpace
      : !(bottomSpace >= tooltipHeight + GAP || bottomSpace >= topSpace)

  tooltipStyle.value = {
    left: left + 'px',
    top: shouldShowOnTop ? rect.top - GAP + 'px' : rect.bottom + GAP + 'px',
    transform: shouldShowOnTop ? 'translate(-50%, -100%)' : 'translate(-50%, 0)',
  }
}

onBeforeUnmount(() => {
  visible.value = false
})
</script>

<template>
  <span
    ref="triggerRef"
    style="display: inline-flex"
    @mouseenter="show"
    @mouseleave="hide"
  >
    <slot />
  </span>
  <Teleport to="body">
    <div
      v-if="visible"
      ref="tooltipRef"
      class="tooltip-floating"
      :style="tooltipStyle"
    >{{ text }}</div>
  </Teleport>
</template>

<style>
.tooltip-floating {
  position: fixed;
  padding: 5px 9px;
  border: 1px solid var(--color-border, #3a3632);
  border-radius: 6px;
  background: var(--color-surface-overlay, #353130);
  color: var(--color-ink, #e8e0d8);
  box-shadow:
    0 8px 20px rgb(0 0 0 / 0.28),
    0 1px 0 rgb(255 255 255 / 0.04) inset;
  font-size: 11px;
  font-weight: 500;
  line-height: 1.4;
  white-space: nowrap;
  pointer-events: none;
  z-index: 9999;
  letter-spacing: 0;
}
</style>
