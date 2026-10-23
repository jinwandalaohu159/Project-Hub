<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { useDismiss } from "../composables/useDismiss";

const VIEWPORT_PADDING = 8;

const props = defineProps<{
  /** Pointer position the menu opens at; `null` keeps it closed. */
  at: { x: number; y: number } | null;
  minWidth?: number;
  /** The button a dropdown menu hangs from: pressing it is not a click outside (it toggles). */
  anchor?: HTMLElement | null;
}>();

const emit = defineEmits<{ close: [] }>();

const menuRef = ref<HTMLElement | null>(null);
const position = ref({ left: 0, top: 0 });

// Opens at the pointer, shifted back inside the window when it would overflow an edge.
watch(
  () => props.at,
  async (at) => {
    if (!at) return;
    position.value = { left: at.x, top: at.y };
    await nextTick();
    const menu = menuRef.value;
    if (!menu) return;
    const maxLeft = window.innerWidth - menu.offsetWidth - VIEWPORT_PADDING;
    const maxTop = window.innerHeight - menu.offsetHeight - VIEWPORT_PADDING;
    position.value = {
      left: Math.max(VIEWPORT_PADDING, Math.min(at.x, maxLeft)),
      top: Math.max(VIEWPORT_PADDING, Math.min(at.y, maxTop)),
    };
  },
  { immediate: true }
);

useDismiss({
  isOpen: () => props.at !== null,
  isInside: (target) => !!(menuRef.value?.contains(target) || props.anchor?.contains(target)),
  onDismiss: () => emit("close"),
  closeOnScroll: true,
  closeOnBlur: true,
});
</script>

<template>
  <Teleport to="body">
    <Transition name="fade">
      <div
        v-if="at"
        ref="menuRef"
        class="context-menu"
        role="menu"
        :style="{ left: `${position.left}px`, top: `${position.top}px`, minWidth: `${minWidth ?? 176}px` }"
        @contextmenu.prevent
      >
        <slot />
      </div>
    </Transition>
  </Teleport>
</template>
