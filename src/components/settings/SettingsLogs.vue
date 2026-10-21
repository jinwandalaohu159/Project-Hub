<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { FileText, FolderOpen, Loader2 } from 'lucide-vue-next'
import { useI18n } from '../../composables/useI18n'
import { useProjectStore } from '../../composables/useProjectStore'
import { useToast } from '../../composables/useToast'
import { APP_LOG_ID, type LogEntry, type LogSettings } from '../../composables/useAppLog'
import { errorText } from '../../utils/errors'
import SelectMenu from '../SelectMenu.vue'
import LogViewer from './LogViewer.vue'
import { isTauri } from '../../utils/env'

const { t } = useI18n()
const { projects, selectedProject, loadProjects } = useProjectStore()
const toast = useToast()

const selectedLogProjectId = ref('')
const logSettings = ref<LogSettings | null>(null)
const logMaxMb = ref(50)
const logRetentionDays = ref(7)
const logSettingsLoading = ref(false)
const logSettingsSaved = ref(false)
const logSettingsLoaded = ref(false)
const showLogViewer = ref(false)
const logEntries = ref<LogEntry[]>([])
const logEntriesLoading = ref(false)
const logEntriesError = ref('')
const openingLogFolder = ref(false)
let logSettingsSaveTimer: number | null = null
let logSettingsSavedTimer: number | null = null

// Project Hub's own log comes first, then one log per project.
const logSourceOptions = computed(() => [
  { value: APP_LOG_ID, label: t('appLogSource') },
  ...projects.value.map((project) => ({ value: project.id, label: project.name })),
])

const selectedLogSourceName = computed(() =>
  logSourceOptions.value.find((option) => option.value === selectedLogProjectId.value)?.label ?? ''
)

function defaultLogProjectId() {
  return selectedProject.value?.id ?? projects.value[0]?.id ?? APP_LOG_ID
}

function ensureLogProjectSelection() {
  const selectedExists = logSourceOptions.value.some((option) => option.value === selectedLogProjectId.value)
  if (!selectedLogProjectId.value || !selectedExists) {
    selectedLogProjectId.value = defaultLogProjectId()
  }
}

async function loadLogSettings() {
  if (!isTauri()) {
    return
  }
  ensureLogProjectSelection()
  if (!selectedLogProjectId.value) {
    logSettings.value = null
    logSettingsLoaded.value = false
    return
  }

  logSettingsLoading.value = true
  logSettingsLoaded.value = false
  try {
    const settings = await invoke<LogSettings>('get_app_log_settings', {
      projectId: selectedLogProjectId.value,
    })
    logSettings.value = settings
    logMaxMb.value = settings.max_mb
    logRetentionDays.value = settings.retention_days
    logSettingsLoaded.value = true
  } catch (error) {
    toast.error(errorText(error, t))
  } finally {
    logSettingsLoading.value = false
  }
}

async function saveLogSettings() {
  if (logSettingsLoading.value) return
  ensureLogProjectSelection()
  if (!selectedLogProjectId.value) return

  logSettingsLoading.value = true
  logSettingsSaved.value = false
  try {
    const settings = await invoke<LogSettings>('set_app_log_settings', {
      projectId: selectedLogProjectId.value,
      maxMb: Math.max(1, Math.round(Number(logMaxMb.value) || 50)),
      retentionDays: Math.max(1, Math.round(Number(logRetentionDays.value) || 7)),
    })
    logSettings.value = settings
    logMaxMb.value = settings.max_mb
    logRetentionDays.value = settings.retention_days
    logSettingsSaved.value = true
    if (logSettingsSavedTimer) clearTimeout(logSettingsSavedTimer)
    logSettingsSavedTimer = window.setTimeout(() => {
      logSettingsSaved.value = false
      logSettingsSavedTimer = null
    }, 1200)
  } catch (error) {
    toast.error(errorText(error, t))
  } finally {
    logSettingsLoading.value = false
  }
}

function scheduleLogSettingsSave() {
  if (!logSettingsLoaded.value || !selectedLogProjectId.value) return
  if (logSettingsSaveTimer) clearTimeout(logSettingsSaveTimer)
  logSettingsSaveTimer = window.setTimeout(() => {
    logSettingsSaveTimer = null
    saveLogSettings()
  }, 500)
}

async function loadLogEntries() {
  if (!isTauri()) {
    return
  }
  ensureLogProjectSelection()
  if (!selectedLogProjectId.value) {
    logEntries.value = []
    return
  }

  logEntriesLoading.value = true
  logEntriesError.value = ''
  try {
    logEntries.value = await invoke<LogEntry[]>('read_app_logs', {
      projectId: selectedLogProjectId.value,
      limit: 300,
    })
  } catch (error) {
    logEntriesError.value = String(error)
  } finally {
    logEntriesLoading.value = false
  }
}

async function openLogViewer() {
  showLogViewer.value = true
  await loadLogEntries()
}

async function openLogFolder() {
  if (!selectedLogProjectId.value || openingLogFolder.value) return
  openingLogFolder.value = true
  try {
    await invoke('open_app_log_dir', {
      projectId: selectedLogProjectId.value,
    })
  } catch (error) {
    toast.error(errorText(error, t))
  } finally {
    window.setTimeout(() => {
      openingLogFolder.value = false
    }, 180)
  }
}

async function onLogProjectChange() {
  if (logSettingsSaveTimer) {
    clearTimeout(logSettingsSaveTimer)
    logSettingsSaveTimer = null
  }
  await loadLogSettings()
  if (showLogViewer.value) {
    await loadLogEntries()
  }
}

watch([projects, selectedProject], () => {
  const before = selectedLogProjectId.value
  ensureLogProjectSelection()
  if (before !== selectedLogProjectId.value && selectedLogProjectId.value) {
    loadLogSettings()
    if (showLogViewer.value) {
      loadLogEntries()
    }
  }
})

watch([logMaxMb, logRetentionDays], scheduleLogSettingsSave)

onMounted(async () => {
  await loadProjects()
  ensureLogProjectSelection()
  await loadLogSettings()
})

onBeforeUnmount(() => {
  if (logSettingsSaveTimer) clearTimeout(logSettingsSaveTimer)
  if (logSettingsSavedTimer) clearTimeout(logSettingsSavedTimer)
})
</script>

<template>
  <div class="setting-group">
    <label class="setting-label">{{ t('logs') }}</label>
    <div class="setting-card">
      <div class="setting-copy">
        <div class="setting-name">{{ t('operationLogs') }}</div>
        <div class="setting-description">{{ t('operationLogsDescription') }}</div>
      </div>
      <div class="select-field">
        <span>{{ t('logProject') }}</span>
        <SelectMenu
          v-model="selectedLogProjectId"
          :options="logSourceOptions"
          @change="onLogProjectChange"
        />
      </div>
      <div class="log-fields">
        <label class="number-field">
          <span>{{ t('logMaxSizeMb') }}</span>
          <input v-model.number="logMaxMb" :disabled="!selectedLogProjectId" type="number" min="1" max="1024" />
        </label>
        <label class="number-field">
          <span>{{ t('logRetentionDays') }}</span>
          <input v-model.number="logRetentionDays" :disabled="!selectedLogProjectId" type="number" min="1" max="3650" />
        </label>
      </div>
      <div class="log-actions">
        <button class="secondary-btn action-btn" :disabled="!selectedLogProjectId || logEntriesLoading" @click="openLogViewer">
          <Loader2 v-if="logEntriesLoading" :size="14" class="spin" />
          <FileText v-else :size="14" />
          {{ t('viewLogs') }}
        </button>
        <button class="secondary-btn action-btn" :disabled="!selectedLogProjectId || openingLogFolder" @click="openLogFolder">
          <Loader2 v-if="openingLogFolder" :size="14" class="spin" />
          <FolderOpen v-else :size="14" />
          {{ t('openLogFolder') }}
        </button>
      </div>
      <Transition name="fade" mode="out-in">
        <p v-if="logSettingsSaved" key="saved" class="setting-success">{{ t('saved') }}</p>
        <p v-else-if="logSettingsLoading" key="saving" class="setting-hint">{{ t('saving') }}</p>
      </Transition>
    </div>
  </div>

  <LogViewer
    :open="showLogViewer"
    :source-name="selectedLogSourceName"
    :entries="logEntries"
    :loading="logEntriesLoading"
    :error="logEntriesError"
    @refresh="loadLogEntries"
    @close="showLogViewer = false"
  />
</template>
