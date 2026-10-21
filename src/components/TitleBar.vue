<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { ArrowDownToLine, Bug, Copy, Loader2, Minus, Settings, Square, X } from 'lucide-vue-next'
import Tooltip from './Tooltip.vue'
import { useI18n } from '../composables/useI18n'
import { useWindowState } from '../composables/useWindowState'
import { useUpdater } from '../composables/useUpdater'
import { isMac, isTauri } from '../utils/env'

defineProps<{
  devPanelOpen: boolean
  settingsOpen: boolean
}>()

const emit = defineEmits<{
  toggleDevPanel: []
  openSettings: []
}>()

// Served from public/; bound dynamically so the bundler does not try to import it.
const logoUrl = '/app-icon.png'

const { t } = useI18n()
// macOS keeps its own traffic-light buttons on the left of the bar; full screen hides them.
const mac = isMac()
const showWindowControls = isTauri() && !mac
const { maximized, fullscreen } = useWindowState()
const { info, available, installing, phase, progress, dialogOpen } = useUpdater()

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
</script>

<template>
  <header class="title-bar" :class="{ 'title-bar-mac': mac && !fullscreen }" data-tauri-drag-region>
    <img :src="logoUrl" alt="" class="title-bar-logo" draggable="false" data-tauri-drag-region />
    <span class="title-bar-name" data-tauri-drag-region>Project Hub</span>
    <div class="flex-1 self-stretch" data-tauri-drag-region />

    <div class="title-bar-actions">
      <Tooltip v-if="available && info" :text="t('updateAvailable', { version: info.latest })" position="bottom">
        <button class="title-bar-update" @click="dialogOpen = true">
          <Loader2 v-if="installing || phase !== 'idle'" :size="13" class="spin" />
          <ArrowDownToLine v-else :size="13" />
          <span>{{
            phase === 'restarting' ? t('updateRestartingShort') : installing ? `${Math.round(progress * 100)}%` : t('updateButton')
          }}</span>
        </button>
      </Tooltip>
      <Tooltip :text="t('debugPanel')" position="bottom">
        <button class="title-bar-btn" :class="{ active: devPanelOpen }" @click="emit('toggleDevPanel')">
          <Bug :size="15" />
        </button>
      </Tooltip>
      <Tooltip :text="t('settings')" position="bottom">
        <button class="title-bar-btn" :class="{ active: settingsOpen }" @click="emit('openSettings')">
          <Settings :size="15" />
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
  transition: padding-left 0.3s ease;
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

.title-bar-update {
  display: flex;
  align-items: center;
  gap: 5px;
  height: 24px;
  margin-right: 4px;
  border-radius: 999px;
  background: var(--color-primary);
  padding: 0 10px 0 8px;
  color: var(--color-on-primary);
  font-size: 12px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  transition: background-color var(--dur-fast, 120ms) ease;
}

.title-bar-update:hover {
  background: var(--color-primary-hover);
}

.title-bar-btn.active {
  background: var(--color-surface-raised);
  color: var(--color-primary);
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
