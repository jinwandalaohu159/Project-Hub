<script setup lang="ts" generic="T extends string | number">
import { computed, nextTick, ref } from "vue";
import { Check, ChevronDown } from "lucide-vue-next";
import { useDismiss } from "../composables/useDismiss";

/** A dropdown in the app's own style, instead of the system's <select>. */
export interface SelectOption<V> {
  value: V;
  label: string;
}

const props = withDefaults(
  defineProps<{
    options: SelectOption<T>[];
    /** Shown while nothing is chosen (or there is nothing to choose). */
    placeholder?: string;
    disabled?: boolean;
    size?: "sm" | "md";
  }>(),
  { placeholder: "", disabled: false, size: "md" }
);

const model = defineModel<T | "">({ required: true });
/** Only when the user picks an option, like <select>'s change event. */
const emit = defineEmits<{ change: [value: T] }>();

const VIEWPORT_PADDING = 8;
const MAX_PANEL_HEIGHT = 280;

const triggerRef = ref<HTMLButtonElement | null>(null);
const panelRef = ref<HTMLElement | null>(null);
const open = ref(false);
/** The option the keyboard is on. */
const active = ref(-1);
const panelStyle = ref<Record<string, string>>({});

const selected = computed(() => props.options.find((option) => option.value === model.value));

/** Below the trigger and as wide as it, or above when there is no room below. */
function place() {
  const rect = triggerRef.value?.getBoundingClientRect();
  if (!rect) return;
  const below = window.innerHeight - rect.bottom - VIEWPORT_PADDING;
  const above = rect.top - VIEWPORT_PADDING;
  const upward = below < 160 && above > below;
  const height = Math.min(MAX_PANEL_HEIGHT, upward ? above : below) - 4;
  panelStyle.value = {
    left: `${rect.left}px`,
    width: `${rect.width}px`,
    maxHeight: `${height}px`,
    ...(upward ? { bottom: `${window.innerHeight - rect.top + 4}px` } : { top: `${rect.bottom + 4}px` }),
  };
}

async function show() {
  if (props.disabled || props.options.length === 0) return;
  place();
  active.value = Math.max(0, props.options.findIndex((option) => option.value === model.value));
  open.value = true;
  await nextTick();
  scrollActiveIntoView();
}

function close() {
  open.value = false;
}

function choose(option: SelectOption<T>) {
  const changed = option.value !== model.value;
  model.value = option.value;
  if (changed) emit("change", option.value);
  close();
  triggerRef.value?.focus();
}

function scrollActiveIntoView() {
  panelRef.value?.querySelector<HTMLElement>(`[data-index="${active.value}"]`)?.scrollIntoView({ block: "nearest" });
}

function onKeydown(event: KeyboardEvent) {
  if (!open.value) {
    if (["ArrowDown", "ArrowUp", "Enter", " "].includes(event.key)) {
      event.preventDefault();
      show();
    }
    return;
  }
  const last = props.options.length - 1;
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    const step = event.key === "ArrowDown" ? 1 : -1;
    active.value = Math.min(last, Math.max(0, active.value + step));
    scrollActiveIntoView();
  } else if (event.key === "Home" || event.key === "End") {
    event.preventDefault();
    active.value = event.key === "Home" ? 0 : last;
    scrollActiveIntoView();
  } else if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    const option = props.options[active.value];
    if (option) choose(option);
  } else if (event.key === "Tab") {
    close();
  }
}

useDismiss({
  isOpen: () => open.value,
  isInside: (target) => !!(panelRef.value?.contains(target) || triggerRef.value?.contains(target)),
  onDismiss: close,
  closeOnScrollAndBlur: true,
});
</script>

<template>
  <button
    ref="triggerRef"
    type="button"
    class="select-trigger"
    :class="[`select-${size}`, { open }]"
    :disabled="disabled || options.length === 0"
    aria-haspopup="listbox"
    :aria-expanded="open"
    @click="open ? close() : show()"
    @keydown="onKeydown"
  >
    <span class="min-w-0 flex-1 truncate text-left" :class="{ 'text-muted': !selected }">
      {{ selected?.label ?? placeholder }}
    </span>
    <ChevronDown :size="size === 'sm' ? 13 : 14" class="select-chevron" />
  </button>
  <Teleport to="body">
    <Transition name="fade">
      <div v-if="open" ref="panelRef" class="select-panel" role="listbox" :style="panelStyle">
        <button
          v-for="(option, index) in options"
          :key="option.value"
          type="button"
          role="option"
          class="select-option"
          :class="{ active: index === active }"
          :data-index="index"
          :aria-selected="option.value === model"
          @mouseenter="active = index"
          @click="choose(option)"
        >
          <span class="min-w-0 flex-1 truncate">{{ option.label }}</span>
          <Check v-if="option.value === model" :size="13" class="shrink-0 text-primary" />
        </button>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.select-trigger {
  display: flex;
  width: 100%;
  align-items: center;
  gap: 6px;
  border: 1px solid var(--color-border);
  border-radius: 7px;
  background: var(--color-canvas);
  color: var(--color-ink);
  text-align: left;
  transition: border-color var(--dur-fast) ease, box-shadow var(--dur-fast) ease;
}

.select-md {
  height: 34px;
  padding: 0 10px;
  font-size: 13px;
}

.select-sm {
  height: 30px;
  padding: 0 9px;
  font-size: 12px;
}

.select-trigger:hover:not(:disabled) {
  border-color: color-mix(in srgb, var(--color-primary) 45%, var(--color-border));
}

.select-trigger:focus-visible,
.select-trigger.open {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 18%, transparent);
  outline: none;
}

.select-trigger:disabled {
  cursor: not-allowed;
  opacity: 0.6;
}

.select-chevron {
  flex: none;
  color: var(--color-muted);
  transition: transform var(--dur-fast) ease;
}

.select-trigger.open .select-chevron {
  transform: rotate(180deg);
}

.select-panel {
  position: fixed;
  z-index: 10001;
  overflow-y: auto;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-surface-overlay);
  padding: 4px;
  box-shadow: 0 12px 30px rgb(0 0 0 / 0.34);
}

.select-option {
  display: flex;
  width: 100%;
  align-items: center;
  gap: 8px;
  border-radius: 5px;
  padding: 7px 10px;
  color: var(--color-ink);
  font-size: 13px;
  text-align: left;
}

.select-option.active {
  background: var(--color-surface-raised);
}
</style>
