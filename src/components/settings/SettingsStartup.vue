<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from '../../composables/useI18n'
import { useToast } from '../../composables/useToast'
import { errorText } from '../../utils/errors'
import { isTauri } from '../../utils/env'

const { t } = useI18n()
const toast = useToast()

const autostartEnabled = ref(false)
const autostartLoading = ref(false)

async function loadAutostart() {
  if (!isTauri()) {
    return
  }

  autostartLoading.value = true

  try {
    autostartEnabled.value = await invoke<boolean>('get_autostart_enabled')
  } catch (error) {
    toast.error(errorText(error, t))
  } finally {
    autostartLoading.value = false
  }
}

async function toggleAutostart() {
  if (autostartLoading.value) {
    return
  }

  const previousEnabled = autostartEnabled.value
  const nextEnabled = !autostartEnabled.value
  autostartEnabled.value = nextEnabled
  autostartLoading.value = true

  try {
    await invoke('set_autostart_enabled', { enabled: nextEnabled })
    autostartEnabled.value = await invoke<boolean>('get_autostart_enabled')
  } catch (error) {
    autostartEnabled.value = previousEnabled
    toast.error(errorText(error, t))
  } finally {
    autostartLoading.value = false
  }
}

onMounted(loadAutostart)
</script>

<template>
  <div class="setting-group">
    <label class="setting-label">{{ t('startup') }}</label>
    <div class="setting-row">
      <div class="setting-copy">
        <div class="setting-name">{{ t('launchOnStartup') }}</div>
        <div class="setting-description">{{ t('launchOnStartupDescription') }}</div>
      </div>
      <button
        class="switch-btn"
        :class="{ active: autostartEnabled }"
        role="switch"
        :aria-checked="autostartEnabled"
        :aria-busy="autostartLoading"
        @click="toggleAutostart"
      >
        <span class="switch-thumb" />
      </button>
    </div>
  </div>
</template>
