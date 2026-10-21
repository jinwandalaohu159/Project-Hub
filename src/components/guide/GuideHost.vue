<script setup lang="ts">
import GuideReader from "./GuideReader.vue";
import PanelResizer from "../PanelResizer.vue";
import { useGuide } from "../../composables/useGuide";
import { useProjectStore } from "../../composables/useProjectStore";
import { clamp, usePersistentRef } from "../../composables/usePersistentRef";
import type { GuideAction } from "../../utils/guide";

/**
 * Where the guide is shown: centered over the blurred app, or docked as a column on its right so
 * the app stays usable beside it. Sits at the end of the app's main row for the docked column.
 */
const emit = defineEmits<{ openSettings: [] }>();

const DEFAULT_DOCK_WIDTH = 420;
/** Room the panels on the left keep while the guide is docked. */
const APP_MIN_WIDTH = 480;

const { open, layout, switchingLayout, close } = useGuide();
const projectStore = useProjectStore();
const dockWidth = usePersistentRef("guide-width", DEFAULT_DOCK_WIDTH);

function onDockResize(dx: number) {
  dockWidth.value = clamp(dockWidth.value + dx, 340, Math.max(360, Math.min(760, window.innerWidth - APP_MIN_WIDTH)));
}

function onKeydown(event: KeyboardEvent) {
  // Escape on an open dropdown inside the guide closes the dropdown only.
  if ((event.target as Element | null)?.closest?.('[aria-expanded="true"]')) return;
  close();
}

/** A button in the guide: get the guide out of the way when it covers the app, then go. */
function onAction(action: GuideAction) {
  if (layout.value === "center") close();
  if (action === "addProject") projectStore.addProject();
  else emit("openSettings");
}
</script>

<template>
  <!-- Opening and closing fade with the blur already in place; moving between layouts is a morph
       (see useGuide), during which these transitions stand aside. -->
  <Teleport to="body">
    <Transition name="guide-modal" :css="!switchingLayout">
      <div v-if="open && layout === 'center'" class="guide-backdrop" @mousedown.self="close" @keydown.esc="onKeydown">
        <GuideReader layout="center" @action="onAction" />
      </div>
    </Transition>
  </Teleport>

  <Transition name="guide-dock" :css="!switchingLayout">
    <div v-if="open && layout === 'docked'" class="guide-dock-row">
      <PanelResizer side="right" @resize="onDockResize" @reset="dockWidth = DEFAULT_DOCK_WIDTH" />
      <aside class="guide-dock" :style="{ width: `${dockWidth}px`, maxWidth: `calc(100vw - ${APP_MIN_WIDTH}px)` }">
        <GuideReader layout="docked" @action="onAction" />
      </aside>
    </div>
  </Transition>
</template>

<style scoped>
.guide-backdrop {
  position: fixed;
  inset: var(--title-bar-height) 0 0 0;
  z-index: 10000;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: var(--color-modal-overlay);
  backdrop-filter: blur(8px) saturate(1.05);
  -webkit-backdrop-filter: blur(8px) saturate(1.05);
}

.guide-dock-row {
  display: flex;
  flex: none;
  min-width: 0;
}

.guide-dock {
  display: flex;
  flex: none;
  flex-direction: column;
  min-width: 0;
  border-left: 1px solid var(--color-border);
}

.guide-dock > * {
  flex: 1;
}

.guide-modal-enter-active,
.guide-modal-leave-active {
  transition: opacity var(--dur-fast) var(--ease-out);
}

.guide-modal-enter-active .guide-reader {
  transition: transform var(--dur) var(--ease-spring), opacity var(--dur-fast) var(--ease-out);
}

.guide-modal-leave-active .guide-reader {
  transition: transform var(--dur-fast) ease, opacity var(--dur-fast) ease;
}

.guide-modal-enter-from,
.guide-modal-leave-to {
  opacity: 0;
}

.guide-modal-enter-from .guide-reader {
  opacity: 0;
  transform: translateY(8px) scale(0.97);
}

.guide-modal-leave-to .guide-reader {
  opacity: 0;
  transform: scale(0.98);
}

.guide-dock-enter-active,
.guide-dock-leave-active {
  transition: opacity var(--dur) var(--ease-out), transform var(--dur) var(--ease-out);
}

.guide-dock-enter-from,
.guide-dock-leave-to {
  opacity: 0;
  transform: translateX(16px);
}
</style>
