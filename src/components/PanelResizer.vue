<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";

const props = withDefaults(defineProps<{
  side?: "left" | "right";
  direction?: "horizontal" | "vertical";
}>(), {
  side: "left",
  direction: "horizontal",
});

const emit = defineEmits<{
  resize: [delta: number];
  /** Double-click restores the default size. */
  reset: [];
}>();

const isDragging = ref(false);
const isHovered = ref(false);
const isVertical = props.direction === "vertical";
let cleanup: (() => void) | null = null;

function onMouseDown(e: MouseEvent) {
  e.preventDefault();
  isDragging.value = true;
  let lastX = e.clientX;
  let lastY = e.clientY;

  function onMouseMove(e: MouseEvent) {
    if (isVertical) {
      const delta = e.clientY - lastY;
      if (delta === 0) return;
      lastY = e.clientY;
      emit("resize", delta);
    } else {
      const delta = e.clientX - lastX;
      if (delta === 0) return;
      lastX = e.clientX;
      emit("resize", props.side === "left" ? delta : -delta);
    }
  }

  function onMouseUp() {
    isDragging.value = false;
    document.removeEventListener("mousemove", onMouseMove);
    document.removeEventListener("mouseup", onMouseUp);
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    document.body.classList.remove("ph-resizing");
    cleanup = null;
  }

  document.body.style.cursor = isVertical ? "row-resize" : "col-resize";
  document.body.style.userSelect = "none";
  document.body.classList.add("ph-resizing");
  document.addEventListener("mousemove", onMouseMove);
  document.addEventListener("mouseup", onMouseUp);
  cleanup = onMouseUp;
}

onBeforeUnmount(() => cleanup?.());
</script>

<template>
  <div
    v-if="isVertical"
    class="resizer-v shrink-0 cursor-row-resize group relative z-10"
    @mouseenter="isHovered = true"
    @mouseleave="isHovered = false"
    @mousedown="onMouseDown"
    @dblclick="emit('reset')"
  >
    <div
      class="absolute inset-x-0 top-1/2 -translate-y-1/2 rounded-full transition-all duration-150"
      :class="{
        'h-[3px] bg-primary/70': isDragging,
        'h-0.5 bg-primary/40': isHovered && !isDragging,
        'h-px bg-border-subtle': !isHovered && !isDragging,
      }"
    />
  </div>
  <div
    v-else
    class="shrink-0 cursor-col-resize group relative w-2 z-10"
    @mouseenter="isHovered = true"
    @mouseleave="isHovered = false"
    @mousedown="onMouseDown"
    @dblclick="emit('reset')"
  >
    <div
      class="absolute inset-y-0 left-1/2 -translate-x-1/2 rounded-full transition-all duration-150"
      :class="{
        'w-[3px] bg-primary/70': isDragging,
        'w-0.5 bg-primary/40': isHovered && !isDragging,
        'w-px bg-border-subtle': !isHovered && !isDragging,
      }"
    />
  </div>
</template>

<style scoped>
/* 8px grab area that takes no layout space between stacked sections. */
.resizer-v {
  height: 8px;
  margin: -4px 0;
}
</style>
