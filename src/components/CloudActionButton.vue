<script setup lang="ts">
import { ref, watch, type Component } from "vue";
import { Loader2 } from "lucide-vue-next";
import Tooltip from "./Tooltip.vue";

/** One of the upload / download buttons: an icon, a label, and a count of records waiting. */
const props = defineProps<{
  icon: Component;
  label: string;
  tooltip: string;
  loading: boolean;
  disabled: boolean;
  /** Records waiting to go up or come down; 0 hides the badge. */
  count?: number;
}>();
defineEmits<{ click: [] }>();

// The badge keeps showing its last number while it folds away, instead of flashing "0".
const shown = ref(props.count ?? 0);
watch(
  () => props.count ?? 0,
  (count) => {
    if (count > 0) shown.value = count;
  },
);
</script>

<template>
  <Tooltip :text="tooltip" position="bottom">
    <button class="cloud-btn" :class="{ pending: (count ?? 0) > 0 }" :disabled="disabled" @click="$emit('click')">
      <Loader2 v-if="loading" :size="14" class="spin" />
      <component :is="icon" v-else :size="14" />
      <span>{{ label }}</span>
      <!-- Stays during the transfer and folds away (width included) once nothing is waiting. -->
      <span class="cloud-badge-slot" :class="{ open: (count ?? 0) > 0 }" aria-hidden="true">
        <span class="cloud-badge-clip">
          <span class="cloud-badge">
            <Transition name="count">
              <span :key="shown" class="cloud-badge-number">{{ shown }}</span>
            </Transition>
          </span>
        </span>
      </span>
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
  transition:
    color var(--dur-fast, 120ms) ease,
    border-color var(--dur-fast, 120ms) ease,
    opacity var(--dur-fast, 120ms) ease;
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

/* Width animates through the grid column, so the button grows and shrinks smoothly instead of
   jumping; the negative margin cancels the button's gap while the badge is folded away. */
.cloud-badge-slot {
  display: inline-grid;
  grid-template-columns: 0fr;
  margin-left: -5px;
  opacity: 0;
  transform: scale(0.85);
  transition:
    grid-template-columns 240ms var(--ease-out, cubic-bezier(0.2, 0.8, 0.2, 1)),
    opacity 180ms ease,
    transform 240ms var(--ease-out, cubic-bezier(0.2, 0.8, 0.2, 1));
}

.cloud-badge-slot.open {
  grid-template-columns: 1fr;
  opacity: 1;
  transform: none;
}

.cloud-badge-clip {
  min-width: 0;
  overflow: hidden;
}

.cloud-badge {
  position: relative;
  display: inline-flex;
  justify-content: center;
  min-width: 18px;
  margin-left: 5px;
  overflow: hidden;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-primary) 22%, transparent);
  padding: 0 5px;
  color: var(--color-primary);
  font-size: 11px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  line-height: 16px;
}

.cloud-badge-number {
  display: inline-block;
}

.count-enter-active,
.count-leave-active {
  transition:
    transform 160ms ease,
    opacity 160ms ease;
}

/* The old number slides out on top while the new one takes its place, so the width never dips. */
.count-leave-active {
  position: absolute;
}

.count-enter-from {
  opacity: 0;
  transform: translateY(60%);
}

.count-leave-to {
  opacity: 0;
  transform: translateY(-60%);
}
</style>
