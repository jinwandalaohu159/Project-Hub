<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Bug, Copy, Minus, Settings, Square, X } from 'lucide-vue-next'
import Tooltip from './Tooltip.vue'
import { useI18n } from '../composables/useI18n'
import { isTauri } from '../utils/env'

defineProps<{
  devPanelOpen: boolean
  settingsOpen: boolean
  updateAvailable: boolean
}>()

const emit = defineEmits<{
  toggleDevPanel: []
  openSettings: []
}>()

// Served from public/; bound dynamically so the bundler does not try to import it.
const logoUrl = '/app-icon.png'

const { t } = useI18n()
const inTauri = isTauri()
// macOS keeps its own traffic-light buttons on the left of the bar.
const isMac = typeof navigator !== 'undefined' && /Mac/i.test(navigator.platform || navigator.userAgent)
const showWindowControls = inTauri && !isMac
const maximized = ref(false)
let unlistenResize: (() => void) | null = null
let syncTimer: ReturnType<typeof setTimeout> | null = null
let disposed = false

async function syncMaximized() {
  try {
    maximized.value = await getCurrentWindow().isMaximized()
  } catch {
    // Window state is cosmetic here.
  }
}

/** Resizing fires many events; only the state after it settles matters for the icon. */
function scheduleSync() {
  if (syncTimer) clearTimeout(syncTimer)
  syncTimer = setTimeout(syncMaximized, 120)
}

function minimize() {
  getCurrentWindow().minimize()
}

function toggleMaximize() {
  getCurrentWindow().toggleMaximize()
}

/** Same as the native close button: the app keeps running in the tray. */
function closeWindow() {
  getCurrentWindow().close()
}

onMounted(async () => {
  if (!showWindowControls) return
  await syncMaximized()
  const unlisten = await getCurrentWindow().onResized(scheduleSync)
  // Unmounted while the listener was being registered.
  if (disposed) unlisten()
  else unlistenResize = unlisten
})

onUnmounted(() => {
  disposed = true
  if (syncTimer) clearTimeout(syncTimer)
  unlistenResize?.()
  unlistenResize = null
})
</script>

<template>
  <header class="title-bar" :class="{ 'title-bar-mac': isMac }" data-tauri-drag-region>
    <img v-if="!isMac" :src="logoUrl" alt="" class="title-bar-logo" draggable="false" data-tauri-drag-region />
    <span class="title-bar-name" data-tauri-drag-region>Project Hub</span>
    <div class="flex-1 self-stretch" data-tauri-drag-region />

    <div class="title-bar-actions">
      <Tooltip :text="t('debugPanel')" position="bottom">
        <button class="title-bar-btn" :class="{ active: devPanelOpen }" @click="emit('toggleDevPanel')">
          <Bug :size="15" />
        </button>
      </Tooltip>
      <Tooltip :text="t('settings')" position="bottom">
        <button class="title-bar-btn" :class="{ active: settingsOpen }" @click="emit('openSettings')">
          <Settings :size="15" />
          <span v-if="updateAvailable" class="title-bar-dot" />
        </button>
      </Tooltip>
    </div>

    <div v-if="showWindowControls" class="window-controls">
      <button class="window-btn" :title="t('minimizeWindow')" @click="minimize">
        <Minus :size="15" />
      </button>
      <button class="window-btn" :title="maximized ? t('restoreWindow') : t('maximizeWindow')" @click="toggleMaximize">
        <Copy v-if="maximized" :size="12" class="-scale-x-100" />
        <Square v-else :size="12" />
      </button>
      <button class="window-btn window-btn-close" :title="t('close')" @click="closeWindow">
        <X :size="16" />
      </button>
    </div>
  </header>
</template>

<style scoped>
.title-bar {
  display: flex;
  flex: none;
  align-items: center;
  height: var(--title-bar-height);
  padding-left: 12px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-canvas);
  user-select: none;
}

/* Room for the macOS traffic lights drawn over the bar. */
.title-bar-mac {
  padding-left: 80px;
}

.title-bar-logo {
  width: 18px;
  height: 18px;
  margin-right: 8px;
  border-radius: 4px;
}

.title-bar-name {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--color-muted);
  letter-spacing: 0.01em;
}

.title-bar-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 0 8px;
}

.title-bar-btn {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 26px;
  border-radius: 6px;
  color: var(--color-muted);
  transition: background-color var(--dur-fast, 120ms) ease, color var(--dur-fast, 120ms) ease;
}

.title-bar-btn:hover {
  background: var(--color-surface-raised);
  color: var(--color-ink);
}

.title-bar-btn.active {
  background: var(--color-surface-raised);
  color: var(--color-primary);
}

.title-bar-dot {
  position: absolute;
  top: 3px;
  right: 5px;
  width: 7px;
  height: 7px;
  border: 1.5px solid var(--color-canvas);
  border-radius: 50%;
  background: var(--color-primary);
}

.window-controls {
  display: flex;
  align-self: stretch;
}

.window-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 46px;
  color: var(--color-muted);
  transition: background-color var(--dur-fast, 120ms) ease, color var(--dur-fast, 120ms) ease;
}

.window-btn:hover {
  background: var(--color-surface-raised);
  color: var(--color-ink);
}

.window-btn-close:hover {
  background: #c42b1c;
  color: #fff;
}
</style>
