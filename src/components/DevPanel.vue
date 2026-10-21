<script setup lang="ts">
import { ref } from 'vue'
import { useProjectStore } from '../composables/useProjectStore'
import { useFileStore } from '../composables/useFileStore'
import { useI18n, type TranslationKey } from '../composables/useI18n'
import { errorText } from '../utils/errors'

defineEmits<{ close: [] }>()

const projectStore = useProjectStore()
const fileStore = useFileStore()
const { t, locale, setLocale } = useI18n()

type Status = 'running' | 'ok' | 'failed' | 'skipped'

interface Check {
  label: TranslationKey
  tone?: 'primary' | 'warning'
  /** Returns what to show on success; throws `Skipped` when there is nothing to check. */
  run: () => string | Promise<string>
}

class Skipped extends Error {}

const STATUS_KEYS: Record<Status, TranslationKey> = {
  running: 'devRunning',
  ok: 'devOk',
  failed: 'devFailed',
  skipped: 'devSkipped',
}
const STATUS_CLASSES: Record<Status, string> = {
  running: 'text-warning',
  ok: 'text-success',
  failed: 'text-danger',
  skipped: 'text-muted',
}

const sections: { title: TranslationKey; checks: Check[] }[] = [
  {
    title: 'devSectionProjects',
    checks: [
      {
        label: 'devAddProject',
        tone: 'primary',
        run: async () => {
          await projectStore.addProject()
          return t('devProjectCount', { count: projectStore.projects.value.length })
        },
      },
      {
        label: 'devReloadProjects',
        run: async () => {
          await projectStore.loadProjects()
          return t('devProjectCount', { count: projectStore.projects.value.length })
        },
      },
      {
        label: 'devSelectFirstProject',
        run: async () => {
          const first = projectStore.projects.value[0]
          if (!first) throw new Skipped(t('devNoProjects'))
          await projectStore.selectProject(first.id)
          return t('devSelected', { name: first.name })
        },
      },
      {
        label: 'devReloadVersions',
        run: async () => {
          await projectStore.loadBranches()
          return t('devVersionCount', { count: projectStore.branches.value.length })
        },
      },
    ],
  },
  {
    title: 'devSectionState',
    checks: [
      {
        label: 'devCheckUnsaved',
        run: async () => t((await projectStore.hasUnsavedChanges()) ? 'devYes' : 'devNo'),
      },
      {
        label: 'devListChanges',
        run: async () => t('devChangeCount', { count: (await projectStore.getUnsavedChanges()).length }),
      },
      {
        label: 'devFileTreeState',
        run: () =>
          t('devFileTree', {
            path: fileStore.selectedFilePath.value || t('devNone'),
            count: fileStore.fileTree.value?.length ?? 0,
          }),
      },
      {
        label: 'devLanguageState',
        run: () => t('devLanguage', { locale: locale.value, sample: `${t('projects')} / ${t('branches')}` }),
      },
      {
        label: 'devSwitchLanguage',
        tone: 'warning',
        run: () => {
          setLocale(locale.value === 'zh' ? 'en' : 'zh')
          return t('devLanguage', { locale: locale.value, sample: `${t('projects')} / ${t('branches')}` })
        },
      },
    ],
  },
]

const results = ref<{ label: TranslationKey; status: Status; detail: string }[]>([])

function report(label: TranslationKey, status: Status, detail = '') {
  const existing = results.value.find((result) => result.label === label)
  if (existing) Object.assign(existing, { status, detail })
  else results.value.push({ label, status, detail })
}

async function run(check: Check) {
  report(check.label, 'running')
  try {
    report(check.label, 'ok', await check.run())
  } catch (error) {
    if (error instanceof Skipped) report(check.label, 'skipped', error.message)
    else report(check.label, 'failed', errorText(error, t))
  }
}

function buttonClass(check: Check) {
  if (check.tone === 'primary') return 'bg-primary text-on-primary hover:bg-primary-hover'
  if (check.tone === 'warning') return 'bg-warning/20 text-warning hover:bg-warning/30'
  return 'bg-surface-raised text-ink hover:bg-surface-overlay'
}
</script>

<template>
  <div class="fixed inset-x-0 bottom-0 top-[var(--title-bar-height)] z-50 flex items-end justify-end" style="background: rgba(10, 8, 6, 0.5)" @click.self="$emit('close')">
    <div class="w-96 max-h-[66vh] bg-surface-overlay border border-border rounded-lg overflow-y-auto">
      <div class="px-4 py-3 border-b border-border flex items-center justify-between">
        <span class="text-base font-semibold text-ink">{{ t('debugPanel') }}</span>
        <button @click="$emit('close')" class="text-muted hover:text-ink text-sm px-2 py-1 rounded-md hover:bg-surface-raised">{{ t('close') }}</button>
      </div>
      <div class="p-4 space-y-3">
        <div v-for="section in sections" :key="section.title" class="space-y-2">
          <h4 class="text-sm font-semibold text-muted uppercase tracking-wider">{{ t(section.title) }}</h4>
          <button
            v-for="check in section.checks"
            :key="check.label"
            class="w-full text-sm py-2 rounded active:scale-95 transition-all"
            :class="buttonClass(check)"
            @click="run(check)"
          >
            {{ t(check.label) }}
          </button>
        </div>
        <div v-if="results.length > 0" class="space-y-1">
          <h4 class="text-sm font-semibold text-muted uppercase tracking-wider">{{ t('devSectionResults') }}</h4>
          <div v-for="result in results" :key="result.label" class="text-sm">
            <span class="text-muted">{{ t(result.label) }} · </span>
            <span :class="STATUS_CLASSES[result.status]">{{ t(STATUS_KEYS[result.status]) }}</span>
            <span v-if="result.detail" class="text-ink"> · {{ result.detail }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
