<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from "vue";
import { X } from "lucide-vue-next";

const props = withDefaults(defineProps<{
  open: boolean;
  title?: string;
  subtitle?: string;
  width?: string;
  /** Prevents closing while an operation is running. */
  busy?: boolean;
  closeOnBackdrop?: boolean;
}>(), {
  width: "460px",
  busy: false,
  closeOnBackdrop: true,
});

const emit = defineEmits<{ close: [] }>();

const panelRef = ref<HTMLElement | null>(null);
let previouslyFocused: Element | null = null;

function requestClose() {
  if (!props.busy) emit("close");
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.stopPropagation();
    requestClose();
  }
}

function onBackdrop() {
  if (props.closeOnBackdrop) requestClose();
}

watch(
  () => props.open,
  async (open) => {
    if (open) {
      previouslyFocused = document.activeElement;
      await nextTick();
      const target =
        panelRef.value?.querySelector<HTMLElement>("[data-autofocus]") ??
        panelRef.value?.querySelector<HTMLElement>("input, textarea, select") ??
        panelRef.value;
      target?.focus();
      if (target instanceof HTMLInputElement) target.select();
    } else if (previouslyFocused instanceof HTMLElement) {
      previouslyFocused.focus();
    }
  },
  { immediate: true }
);

onBeforeUnmount(() => {
  if (previouslyFocused instanceof HTMLElement) previouslyFocused.focus();
});
</script>

<template>
  <Teleport to="body">
    <Transition name="modal" appear>
      <div v-if="open" class="modal-backdrop" @mousedown.self="onBackdrop" @keydown="onKeydown">
        <div
          ref="panelRef"
          class="modal-panel"
          role="dialog"
          aria-modal="true"
          tabindex="-1"
          :style="{ width }"
        >
          <header v-if="title" class="modal-header">
            <div class="min-w-0">
              <h3 class="modal-title">{{ title }}</h3>
              <p v-if="subtitle" class="modal-subtitle">{{ subtitle }}</p>
            </div>
            <button class="modal-close" :disabled="busy" @click="requestClose">
              <X :size="16" />
            </button>
          </header>
          <div class="modal-body">
            <slot />
          </div>
          <footer v-if="$slots.footer" class="modal-footer">
            <slot name="footer" />
          </footer>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-backdrop {
  position: fixed;
  inset: 0;
  z-index: 10000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: var(--color-modal-overlay);
  backdrop-filter: blur(8px) saturate(1.05);
  -webkit-backdrop-filter: blur(8px) saturate(1.05);
}

.modal-panel {
  display: flex;
  flex-direction: column;
  max-width: 100%;
  max-height: min(86vh, 760px);
  overflow: hidden;
  border: 1px solid var(--color-border);
  border-radius: 14px;
  background: var(--color-surface);
  box-shadow: 0 24px 64px rgb(0 0 0 / 0.38), 0 1px 0 rgb(255 255 255 / 0.04) inset;
  outline: none;
}

.modal-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding: 18px 20px 6px;
}

.modal-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--color-ink);
}

.modal-subtitle {
  margin-top: 3px;
  font-size: 12px;
  line-height: 1.5;
  color: var(--color-muted);
  word-break: break-word;
}

.modal-close {
  flex: none;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  margin: -4px -6px 0 0;
  border-radius: 7px;
  color: var(--color-muted);
  transition: background-color var(--dur-fast) var(--ease-out), color var(--dur-fast) var(--ease-out);
}

.modal-close:hover:not(:disabled) {
  background: var(--color-surface-raised);
  color: var(--color-ink);
}

.modal-body {
  min-height: 0;
  overflow-y: auto;
  padding: 12px 20px 16px;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 20px 16px;
  border-top: 1px solid var(--color-border-subtle);
  background: color-mix(in srgb, var(--color-surface) 70%, var(--color-canvas));
}

.modal-enter-active,
.modal-leave-active {
  transition: opacity var(--dur) var(--ease-out), backdrop-filter var(--dur) var(--ease-out);
}

.modal-enter-active .modal-panel {
  transition: transform var(--dur-slow) var(--ease-spring), opacity var(--dur) var(--ease-out);
}

.modal-leave-active .modal-panel {
  transition: transform var(--dur-fast) ease, opacity var(--dur-fast) ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
  backdrop-filter: blur(0);
}

.modal-enter-from .modal-panel {
  opacity: 0;
  transform: translateY(10px) scale(0.965);
}

.modal-leave-to .modal-panel {
  opacity: 0;
  transform: scale(0.98);
}
</style>
