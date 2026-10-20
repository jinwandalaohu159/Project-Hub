<script setup lang="ts">
import { ref } from 'vue'
import { useProjectStore } from '../composables/useProjectStore'
import { useFileStore } from '../composables/useFileStore'
import { useI18n } from '../composables/useI18n'

const projectStore = useProjectStore()
const fileStore = useFileStore()
const { t, locale, setLocale } = useI18n()

const testResults = ref<Record<string, string>>({})

async function testAddProject() {
  testResults.value['addProject'] = 'running...'
  try {
    await projectStore.addProject()
    testResults.value['addProject'] = `ok - ${projectStore.projects.value.length} projects`
  } catch (e) {
    testResults.value['addProject'] = `fail - ${e}`
  }
}

async function testLoadProjects() {
  testResults.value['loadProjects'] = 'running...'
  try {
    await projectStore.loadProjects()
    testResults.value['loadProjects'] = `ok - ${projectStore.projects.value.length} projects`
  } catch (e) {
    testResults.value['loadProjects'] = `fail - ${e}`
  }
}

async function testSelectProject() {
  testResults.value['selectProject'] = 'running...'
  try {
    const first = projectStore.projects.value[0]
    if (!first) {
      testResults.value['selectProject'] = 'skip - no projects'
      return
    }
    await projectStore.selectProject(first.id)
    testResults.value['selectProject'] = `ok - selected ${first.name}`
  } catch (e) {
    testResults.value['selectProject'] = `fail - ${e}`
  }
}

async function testLoadBranches() {
  testResults.value['loadBranches'] = 'running...'
  try {
    await projectStore.loadBranches()
    testResults.value['loadBranches'] = `ok - ${projectStore.branches.value.length} branches`
  } catch (e) {
    testResults.value['loadBranches'] = `fail - ${e}`
  }
}

function testSwitchLocale() {
  const current = locale.value
  const next = current === 'zh' ? 'en' : 'zh'
  setLocale(next)
  testResults.value['switchLocale'] = `ok - switched to ${next}`
}

async function testHasUnsavedChanges() {
  testResults.value['hasUnsavedChanges'] = 'running...'
  try {
    const result = await projectStore.hasUnsavedChanges()
    testResults.value['hasUnsavedChanges'] = `ok - ${result}`
  } catch (e) {
    testResults.value['hasUnsavedChanges'] = `fail - ${e}`
  }
}

async function testGetUnsavedChanges() {
  testResults.value['getUnsavedChanges'] = 'running...'
  try {
    const result = await projectStore.getUnsavedChanges()
    testResults.value['getUnsavedChanges'] = `ok - ${result.length} changes`
  } catch (e) {
    testResults.value['getUnsavedChanges'] = `fail - ${e}`
  }
}

function testFileStoreState() {
  const state = {
    selectedFilePath: fileStore.selectedFilePath.value,
    fileTree: fileStore.fileTree.value?.length ?? 0,
  }
  testResults.value['fileStoreState'] = `ok - path: ${state.selectedFilePath || 'none'}, tree: ${state.fileTree} items`
}

function testI18nState() {
  testResults.value['i18nState'] = `ok - locale: ${locale.value}, projects: "${t('projects')}", branches: "${t('branches')}"`
}
</script>

<template>
  <div class="fixed inset-x-0 bottom-0 top-[var(--title-bar-height)] z-50 flex items-end justify-end" style="background: rgba(10, 8, 6, 0.5)" @click.self="$emit('close')">
    <div class="w-96 max-h-[66vh] bg-surface-overlay border border-border rounded-lg overflow-y-auto">
      <div class="px-4 py-3 border-b border-border flex items-center justify-between">
        <span class="text-base font-semibold text-ink">调试面板</span>
        <button @click="$emit('close')" class="text-muted hover:text-ink text-sm px-2 py-1 rounded-md hover:bg-surface-raised">关闭</button>
      </div>
      <div class="p-4 space-y-3">
        <div class="space-y-2">
          <h4 class="text-sm font-semibold text-muted uppercase tracking-wider">项目检查</h4>
          <button @click="testAddProject" class="w-full text-sm py-2 rounded bg-primary text-on-primary hover:bg-primary-hover active:scale-95 transition-all">添加项目</button>
          <button @click="testLoadProjects" class="w-full text-sm py-2 rounded bg-surface-raised text-ink hover:bg-surface-overlay active:scale-95 transition-all">刷新项目列表</button>
          <button @click="testSelectProject" class="w-full text-sm py-2 rounded bg-surface-raised text-ink hover:bg-surface-overlay active:scale-95 transition-all">选择第一个项目</button>
          <button @click="testLoadBranches" class="w-full text-sm py-2 rounded bg-surface-raised text-ink hover:bg-surface-overlay active:scale-95 transition-all">刷新版本列表</button>
        </div>
        <div class="space-y-2">
          <h4 class="text-sm font-semibold text-muted uppercase tracking-wider">状态检查</h4>
          <button @click="testHasUnsavedChanges" class="w-full text-sm py-2 rounded bg-surface-raised text-ink hover:bg-surface-overlay active:scale-95 transition-all">检查未保存更改</button>
          <button @click="testGetUnsavedChanges" class="w-full text-sm py-2 rounded bg-surface-raised text-ink hover:bg-surface-overlay active:scale-95 transition-all">读取更改列表</button>
          <button @click="testFileStoreState" class="w-full text-sm py-2 rounded bg-surface-raised text-ink hover:bg-surface-overlay active:scale-95 transition-all">查看文件树状态</button>
          <button @click="testI18nState" class="w-full text-sm py-2 rounded bg-surface-raised text-ink hover:bg-surface-overlay active:scale-95 transition-all">查看语言状态</button>
          <button @click="testSwitchLocale" class="w-full text-sm py-2 rounded bg-warning/20 text-warning hover:bg-warning/30 active:scale-95 transition-all">切换语言</button>
        </div>
        <div v-if="Object.keys(testResults).length > 0" class="space-y-1">
          <h4 class="text-sm font-semibold text-muted uppercase tracking-wider">结果</h4>
          <div v-for="[key, value] in Object.entries(testResults)" :key="key" class="text-sm font-mono">
            <span class="text-muted">{{ key }}:</span>
            <span :class="value.startsWith('ok') ? 'text-success' : value.startsWith('fail') ? 'text-danger' : 'text-warning'">{{ value }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
