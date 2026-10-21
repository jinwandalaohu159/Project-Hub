<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useProjectStore } from './composables/useProjectStore'
import { useFileStore } from './composables/useFileStore'
import { useI18n } from './composables/useI18n'
import { useProjectEvents } from './composables/useProjectEvents'
import { useTheme } from './composables/useTheme'
import { usePanelLayout } from './composables/usePanelLayout'
import ProjectPanel from './components/ProjectPanel.vue'
import FileTree from './components/FileTree.vue'
import CodeViewer from './components/CodeViewer.vue'
import PanelResizer from './components/PanelResizer.vue'
import CollapsiblePanel from './components/CollapsiblePanel.vue'
import SettingsOverlay from './components/SettingsOverlay.vue'
import GuideHost from './components/guide/GuideHost.vue'
import ToastHost from './components/ToastHost.vue'
import ConfirmHost from './components/ConfirmHost.vue'
import GithubLoginDialog from './components/GithubLoginDialog.vue'
import UpdateBanner from './components/UpdateBanner.vue'
import UpdateDialog from './components/UpdateDialog.vue'
import TitleBar from './components/TitleBar.vue'
import { useUpdater } from './composables/useUpdater'
import { Plus, Columns2, Rows2 } from 'lucide-vue-next'
import Tooltip from './components/Tooltip.vue'
import { isTauri } from './utils/env'

const projectStore = useProjectStore()
const fileStore = useFileStore()
const { t } = useI18n()
const { notifyVersionContextChanged } = useProjectEvents()
useTheme()
const updater = useUpdater()

const showSettings = ref(false)
let unlistenOpenSettings: UnlistenFn | null = null
let unlistenRepoChanged: UnlistenFn | null = null

const {
  leftWidth,
  leftCollapsed,
  middleCollapsed,
  rightCollapsed,
  leftLayout,
  stacked,
  projectCollapsed,
  filesCollapsed,
  projectPanelStyle,
  filesPanelStyle,
  leftCollapsedVertical,
  middleCollapsedVertical,
  verticalContainerRef,
  onLeftResize,
  onMiddleResize,
  onVerticalResize,
  resetLeftWidth,
  resetMiddleWidth,
  resetVerticalSplit,
  toggleLeftLayout,
} = usePanelLayout()

const codePanelTitle = computed(() => {
  const path = fileStore.selectedFilePath.value
  if (path) {
    const parts = path.split('/')
    return parts[parts.length - 1] || t('code')
  }
  return t('code')
})

const toolbarButtonClass = 'w-8 h-8 flex items-center justify-center rounded-md text-muted hover:text-ink hover:bg-surface-raised transition-colors'

onMounted(async () => {
  if (!isTauri()) {
    return
  }

  updater.startAutoCheck()

  unlistenOpenSettings = await listen('open-settings', () => {
    showSettings.value = true
  })
  // Saves made by agents through the MCP server.
  unlistenRepoChanged = await listen<{ project_id: string }>('repo-changed', async (event) => {
    if (event.payload?.project_id === projectStore.selectedProjectId.value) {
      await projectStore.loadBranches()
      notifyVersionContextChanged()
    }
  })
})

onUnmounted(() => {
  unlistenOpenSettings?.()
  unlistenOpenSettings = null
  unlistenRepoChanged?.()
  unlistenRepoChanged = null
  updater.stopAutoCheck()
})
</script>

<template>
  <div class="flex h-full w-full flex-col">
  <!-- Custom title bar: global actions sit next to the window buttons, whatever panels are collapsed. -->
  <TitleBar
    :settings-open="showSettings"
    @open-settings="showSettings = true"
  />
  <UpdateBanner />
  <UpdateDialog />
  <div class="flex min-h-0 w-full flex-1">
    <!-- Left panels: projects and files, side by side or stacked. Both arrangements are one tree,
         so switching keeps every panel (and what it has loaded) and only moves it. -->
    <div
      ref="verticalContainerRef"
      :data-layout="leftLayout"
      class="flex min-h-0 shrink-0"
      :class="stacked ? 'flex-col border-r border-border panel-size-transition' : 'flex-row'"
      :style="stacked ? { width: leftWidth + 'px' } : undefined"
    >
      <CollapsiblePanel
        v-model:collapsed="projectCollapsed"
        :title="t('projects')"
        side="left"
        :style="projectPanelStyle"
        class="vt-panel-projects panel-size-transition"
        :class="stacked ? 'border-b border-border' : 'shrink-0 border-r border-border'"
      >
        <template #title-actions>
          <Tooltip :text="stacked ? t('sideBySideLayout') : t('stackPanelsVertically')">
            <button
              @click.stop="toggleLeftLayout"
              :class="toolbarButtonClass"
            >
              <Rows2 v-if="stacked" :size="17" />
              <Columns2 v-else :size="17" />
            </button>
          </Tooltip>
          <Tooltip :text="t('addProject')">
            <button
              @click.stop="projectStore.addProject"
              :class="toolbarButtonClass"
            >
              <Plus :size="17" />
            </button>
          </Tooltip>
        </template>
        <ProjectPanel />
      </CollapsiblePanel>
      <PanelResizer
        v-if="stacked && !leftCollapsedVertical && !middleCollapsedVertical"
        direction="vertical"
        @resize="onVerticalResize"
        @reset="resetVerticalSplit"
      />
      <PanelResizer v-else-if="!stacked && !leftCollapsed" side="left" @resize="onLeftResize" @reset="resetLeftWidth" />

      <CollapsiblePanel
        v-model:collapsed="filesCollapsed"
        :title="t('files')"
        side="left"
        :style="filesPanelStyle"
        class="vt-panel-files panel-size-transition"
        :class="stacked ? '' : 'shrink-0 border-r border-border'"
      >
        <FileTree />
      </CollapsiblePanel>
      <PanelResizer v-if="!stacked && !middleCollapsed" side="left" @resize="onMiddleResize" @reset="resetMiddleWidth" />
    </div>
    <PanelResizer v-if="stacked" side="left" @resize="onLeftResize" @reset="resetLeftWidth" />

    <!-- Right Panel: CodeViewer -->
    <CollapsiblePanel
      v-model:collapsed="rightCollapsed"
      :title="codePanelTitle"
      side="right"
      class="vt-panel-code"
      :class="rightCollapsed ? 'w-10 shrink-0' : 'flex-1 min-w-0'"
    >
      <CodeViewer />
    </CollapsiblePanel>

    <!-- User guide: centered over the app, or docked here as a column on the right. -->
    <GuideHost @open-settings="showSettings = true" />

    <!-- Settings overlay -->
    <Transition name="settings">
      <SettingsOverlay v-if="showSettings" @close="showSettings = false" />
    </Transition>

    <ConfirmHost />
    <GithubLoginDialog />
    <ToastHost />
  </div>
  </div>
</template>
