<script setup lang="ts">
import { RefreshCw, X } from 'lucide-vue-next'
import { useI18n } from '../../composables/useI18n'
import type { LogEntry } from '../../composables/useAppLog'

/** Read-only list of one log's entries, newest first. */
defineProps<{
  open: boolean
  /** Which log is shown: a project name or the Project Hub app log. */
  sourceName: string
  entries: LogEntry[]
  loading: boolean
  error: string
}>()

const emit = defineEmits<{ refresh: []; close: [] }>()

const { t } = useI18n()

function formatLogTime(timestampMs: number) {
  return new Date(timestampMs).toLocaleString()
}

function levelLabel(level: string) {
  if (level === 'error') return t('logLevelError')
  if (level === 'warn') return t('logLevelWarn')
  return ''
}
</script>

<template>
  <!-- Fixed overlay; it still sits inside .settings-overlay so the drawer styles apply. -->
  <Transition name="fade">
    <div v-if="open" class="log-viewer-overlay" @click.self="emit('close')">
      <div class="log-viewer">
        <div class="log-viewer-header">
          <div>
            <h3>{{ t('operationLogs') }}</h3>
            <p>{{ sourceName || t('unknown') }}</p>
          </div>
          <div class="log-viewer-actions">
            <button class="icon-btn" @click="emit('refresh')" :title="t('refreshLogs')">
              <RefreshCw :size="15" :class="{ spin: loading }" />
            </button>
            <button class="icon-btn" @click="emit('close')" :title="t('closeSettings')">
              <X :size="15" />
            </button>
          </div>
        </div>
        <div class="log-viewer-body">
          <div v-if="loading" class="log-empty">{{ t('loading') }}</div>
          <div v-else-if="error" class="log-error">{{ error }}</div>
          <div v-else-if="entries.length === 0" class="log-empty">{{ t('noLogs') }}</div>
          <div v-else class="log-list">
            <div v-for="entry in entries" :key="`${entry.timestamp_ms}-${entry.action}-${entry.message}`" class="log-entry">
              <div class="log-entry-meta">
                <span>{{ formatLogTime(entry.timestamp_ms) }}</span>
                <span v-if="levelLabel(entry.level)" class="log-level" :class="`log-level-${entry.level}`">
                  {{ levelLabel(entry.level) }}
                </span>
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
