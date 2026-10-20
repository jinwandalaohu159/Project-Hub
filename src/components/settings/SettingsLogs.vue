<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { FileText, FolderOpen, Loader2, RefreshCw, X } from 'lucide-vue-next'
import { useI18n } from '../../composables/useI18n'
import { useProjectStore } from '../../composables/useProjectStore'
import { useToast } from '../../composables/useToast'
import type { AppLogEntry, AppLogSettings } from '../../composables/useAppLog'
import { errorText } from '../../utils/errors'
import { isTauri } from '../../utils/env'

const { t } = useI18n()
const { projects, selectedProject, loadProjects } = useProjectStore()
const toast = useToast()

const selectedLogProjectId = ref('')
const logSettings = ref<AppLogSettings | null>(null)
const logMaxMb = ref(50)
const logRetentionDays = ref(7)
const logSettingsLoading = ref(false)
const logSettingsSaved = ref(false)
const logSettingsLoaded = ref(false)
const showLogViewer = ref(false)
const appLogs = ref<AppLogEntry[]>([])
const appLogsLoading = ref(false)
const appLogsError = ref('')
const openingLogFolder = ref(false)
let logSettingsSaveTimer: number | null = null
let logSettingsSavedTimer: number | null = null

const selectedLogProject = computed(() =>
  projects.value.find((project) => project.id === selectedLogProjectId.value) ?? null
)

function defaultLogProjectId() {
  return selectedProject.value?.id ?? projects.value[0]?.id ?? ''
}

function ensureLogProjectSelection() {
  const fallbackId = defaultLogProjectId()
  const selectedExists = projects.value.some((project) => project.id === selectedLogProjectId.value)
  if (!selectedLogProjectId.value || !selectedExists) {
    selectedLogProjectId.value = fallbackId
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
    const settings = await invoke<AppLogSettings>('get_app_log_settings', {
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
    const settings = await invoke<AppLogSettings>('set_app_log_settings', {
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

async function loadAppLogs() {
  if (!isTauri()) {
    return
  }
  ensureLogProjectSelection()
  if (!selectedLogProjectId.value) {
    appLogs.value = []
    return
  }

  appLogsLoading.value = true
  appLogsError.value = ''
  try {
    appLogs.value = await invoke<AppLogEntry[]>('read_app_logs', {
      projectId: selectedLogProjectId.value,
      limit: 300,
    })
  } catch (error) {
    appLogsError.value = String(error)
  } finally {
    appLogsLoading.value = false
  }
}

async function openLogViewer() {
  showLogViewer.value = true
  await loadAppLogs()
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
    await loadAppLogs()
  }
}

function formatLogTime(timestampMs: number) {
  return new Date(timestampMs).toLocaleString()
}

watch([projects, selectedProject], () => {
  const before = selectedLogProjectId.value
  ensureLogProjectSelection()
  if (before !== selectedLogProjectId.value && selectedLogProjectId.value) {
    loadLogSettings()
    if (showLogViewer.value) {
      loadAppLogs()
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
      <label class="select-field">
        <span>{{ t('logProject') }}</span>
        <select
          v-model="selectedLogProjectId"
          :disabled="projects.length === 0"
          @change="onLogProjectChange"
        >
          <option v-if="projects.length === 0" value="">{{ t('noProjectForLogs') }}</option>
          <option v-for="project in projects" :key="project.id" :value="project.id">
            {{ project.name }}
          </option>
        </select>
      </label>
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
        <button class="secondary-btn action-btn" :disabled="!selectedLogProjectId || appLogsLoading" @click="openLogViewer">
          <Loader2 v-if="appLogsLoading" :size="14" class="spinning" />
          <FileText v-else :size="14" />
          {{ t('viewLogs') }}
        </button>
        <button class="secondary-btn action-btn" :disabled="!selectedLogProjectId || openingLogFolder" @click="openLogFolder">
          <Loader2 v-if="openingLogFolder" :size="14" class="spinning" />
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

  <!-- Fixed overlay; it still sits inside .settings-overlay so the drawer styles apply. -->
  <Transition name="fade">
    <div v-if="showLogViewer" class="log-viewer-overlay" @click.self="showLogViewer = false">
      <div class="log-viewer">
        <div class="log-viewer-header">
          <div>
            <h3>{{ t('operationLogs') }}</h3>
            <p>{{ selectedLogProject?.name || t('unknown') }}</p>
          </div>
          <div class="log-viewer-actions">
            <button class="icon-btn" @click="loadAppLogs" :title="t('refreshLogs')">
              <RefreshCw :size="15" :class="{ spinning: appLogsLoading }" />
            </button>
            <button class="icon-btn" @click="showLogViewer = false" :title="t('closeSettings')">
              <X :size="15" />
            </button>
          </div>
        </div>
        <div class="log-viewer-body">
          <div v-if="appLogsLoading" class="log-empty">{{ t('loading') }}</div>
          <div v-else-if="appLogsError" class="log-error">{{ appLogsError }}</div>
          <div v-else-if="appLogs.length === 0" class="log-empty">{{ t('noLogs') }}</div>
          <div v-else class="log-list">
            <div v-for="entry in appLogs" :key="`${entry.timestamp_ms}-${entry.action}-${entry.message}`" class="log-entry">
              <div class="log-entry-meta">
                <span>{{ formatLogTime(entry.timestamp_ms) }}</span>
                <span>{{ entry.action }}</span>
              </div>
              <div class="log-entry-message">{{ entry.message }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>
