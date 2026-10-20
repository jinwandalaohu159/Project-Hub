<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from '../composables/useI18n'
import { ChevronRight } from 'lucide-vue-next'
import Tooltip from './Tooltip.vue'

const props = defineProps<{
  collapsed: boolean
  title: string
  side?: 'left' | 'right'
}>()

const emit = defineEmits<{
  'update:collapsed': [value: boolean]
}>()

const { t } = useI18n()

// One icon rotated between states animates instead of swapping glyphs.
const chevronRotation = computed(() => {
  if (!props.collapsed) return 'rotate-90'
  return props.side === 'right' ? 'rotate-180' : 'rotate-0'
})

const collapseTitle = computed(() => props.collapsed ? t('expand') : t('collapse'))

function toggle() {
  emit('update:collapsed', !props.collapsed)
}
</script>

<template>
  <div class="flex flex-col h-full overflow-hidden bg-surface" :class="{ 'shrink-0': collapsed }">
    <Tooltip v-if="collapsed" :text="collapseTitle">
      <button
        data-testid="collapse-btn"
        class="h-full w-10 flex items-start justify-center pt-3 border-x border-border-subtle text-muted hover:text-ink hover:bg-surface-raised transition-colors"
        @click.stop="toggle"
      >
        <ChevronRight :size="18" class="transition-transform duration-300" :class="chevronRotation" />
      </button>
    </Tooltip>
    <div
      v-else
      data-testid="panel-title-bar"
      class="flex items-center justify-between px-3 py-2.5 border-b border-border-subtle cursor-pointer select-none shrink-0 bg-surface hover:bg-surface-raised transition-colors"
      @click="toggle"
    >
      <div class="flex items-center gap-2 min-w-0 flex-1">
        <slot name="title-actions" />
        <span class="text-sm font-semibold text-panel-title uppercase tracking-wider truncate">{{ title }}</span>
      </div>
      <Tooltip :text="collapseTitle">
        <button
          data-testid="collapse-btn"
          class="flex items-center justify-center w-8 h-8 rounded-md text-muted hover:text-ink hover:bg-surface-raised transition-colors shrink-0"
          @click.stop="toggle"
        >
          <ChevronRight :size="18" class="transition-transform duration-300" :class="chevronRotation" />
        </button>
      </Tooltip>
    </div>
    <Transition name="panel-body">
      <div v-show="!collapsed" class="flex-1 min-h-0 overflow-hidden">
        <slot />
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.panel-body-enter-active,
.panel-body-leave-active {
  transition: opacity var(--dur) var(--ease-out), transform var(--dur) var(--ease-out);
}

.panel-body-enter-from,
.panel-body-leave-to {
  opacity: 0;
  transform: translateY(-3px);
}
</style>
