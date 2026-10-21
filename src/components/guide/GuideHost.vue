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

const { open, layout, close } = useGuide();
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
  <Teleport to="body">
    <div v-if="open && layout === 'center'" class="guide-backdrop" @mousedown.self="close" @keydown.esc="onKeydown">
      <GuideReader layout="center" @action="onAction" />
    </div>
  </Teleport>

  <template v-if="open && layout === 'docked'">
    <PanelResizer side="right" @resize="onDockResize" @reset="dockWidth = DEFAULT_DOCK_WIDTH" />
    <aside class="guide-dock" :style="{ width: `${dockWidth}px`, maxWidth: `calc(100vw - ${APP_MIN_WIDTH}px)` }">
      <GuideReader layout="docked" @action="onAction" />
    </aside>
  </template>
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
</style>
