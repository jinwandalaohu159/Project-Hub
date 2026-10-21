<script setup lang="ts">
import { useI18n } from '../composables/useI18n'
import { usePersistentRef, clamp } from '../composables/usePersistentRef'
import { X } from 'lucide-vue-next'
import PanelResizer from './PanelResizer.vue'
import SettingsGeneral from './settings/SettingsGeneral.vue'
import SettingsCodeTheme from './settings/SettingsCodeTheme.vue'
import SettingsUpdate from './settings/SettingsUpdate.vue'
import SettingsAccount from './settings/SettingsAccount.vue'
import SettingsMcp from './settings/SettingsMcp.vue'
import SettingsStartup from './settings/SettingsStartup.vue'
import SettingsLogs from './settings/SettingsLogs.vue'
import SettingsTransfer from './settings/SettingsTransfer.vue'
import './settings/settings.css'

const DEFAULT_PANEL_WIDTH = 380

const emit = defineEmits<{
  close: []
}>()

const { t } = useI18n()
const panelWidth = usePersistentRef('settings-width', DEFAULT_PANEL_WIDTH)

function onPanelResize(dx: number) {
  panelWidth.value = clamp(panelWidth.value + dx, 320, Math.max(360, Math.min(820, window.innerWidth - 80)))
}
</script>

<template>
  <div class="settings-overlay" @mousedown.self="emit('close')" @keydown.esc="emit('close')">
    <PanelResizer side="right" class="settings-resizer" @resize="onPanelResize" @reset="panelWidth = DEFAULT_PANEL_WIDTH" />
    <div class="settings-panel" :style="{ width: `${panelWidth}px` }" tabindex="-1">
      <div class="settings-header">
        <h2 class="settings-title">{{ t('settingsTitle') }}</h2>
        <button class="close-btn" @click="emit('close')" :title="t('closeSettings')">
          <X :size="16" />
        </button>
      </div>
      <div class="settings-body">
        <SettingsGeneral />

        <SettingsCodeTheme />

        <SettingsUpdate />

        <SettingsAccount />

        <SettingsMcp />

        <SettingsStartup />

        <SettingsLogs />

        <SettingsTransfer />
      </div>
    </div>
  </div>
</template>
