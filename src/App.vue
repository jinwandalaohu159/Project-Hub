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
  middleWidth,
  leftCollapsed,
  middleCollapsed,
  rightCollapsed,
  leftLayout,
  leftCollapsedVertical,
  middleCollapsedVertical,
  verticalSplit,
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

const verticalFileOffsetPx = 10
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
    <!-- Left panels area -->
    <template v-if="leftLayout === 'horizontal'">
      <!-- Left Panel: ProjectPanel -->
      <CollapsiblePanel
        v-model:collapsed="leftCollapsed"
        :title="t('projects')"
        side="left"
        :style="{ width: leftCollapsed ? '40px' : leftWidth + 'px' }"
        class="shrink-0 border-r border-border panel-size-transition"
      >
        <template #title-actions>
          <Tooltip :text="t('stackPanelsVertically')">
            <button
              @click.stop="toggleLeftLayout"
              :class="toolbarButtonClass"
            >
              <Columns2 :size="17" />
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
      <PanelResizer v-if="!leftCollapsed" side="left" @resize="onLeftResize" @reset="resetLeftWidth" />

      <!-- Middle Panel: FileTree -->
      <CollapsiblePanel
        v-model:collapsed="middleCollapsed"
        :title="t('files')"
        side="left"
        :style="{ width: middleCollapsed ? '40px' : middleWidth + 'px' }"
        class="shrink-0 border-r border-border panel-size-transition"
      >
        <FileTree />
      </CollapsiblePanel>
      <PanelResizer v-if="!middleCollapsed" side="left" @resize="onMiddleResize" @reset="resetMiddleWidth" />
    </template>

    <template v-else>
      <!-- Vertical layout: both panels stacked -->
      <div
        ref="verticalContainerRef"
        class="flex flex-col shrink-0 border-r border-border panel-size-transition"
        :style="{ width: leftWidth + 'px' }"
      >
        <CollapsiblePanel
          v-model:collapsed="leftCollapsedVertical"
          :title="t('projects')"
          side="left"
          :style="leftCollapsedVertical ? { height: '40px' } : middleCollapsedVertical ? { flex: 1 } : { height: `calc(${verticalSplit}% - ${verticalFileOffsetPx}px)` }"
          class="border-b border-border panel-size-transition"
        >
          <template #title-actions>
            <Tooltip :text="t('sideBySideLayout')">
              <button
                @click.stop="toggleLeftLayout"
                :class="toolbarButtonClass"
              >
                <Rows2 :size="17" />
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
          v-if="!leftCollapsedVertical && !middleCollapsedVertical"
          direction="vertical"
          @resize="onVerticalResize"
          @reset="resetVerticalSplit"
        />
        <CollapsiblePanel
          v-model:collapsed="middleCollapsedVertical"
          :title="t('files')"
          side="left"
          :style="middleCollapsedVertical ? { height: '40px' } : { flex: 1 }"
          class="panel-size-transition"
        >
          <FileTree />
        </CollapsiblePanel>
      </div>
      <PanelResizer side="left" @resize="onLeftResize" @reset="resetLeftWidth" />
    </template>

    <!-- Right Panel: CodeViewer -->
    <CollapsiblePanel
      v-model:collapsed="rightCollapsed"
      :title="codePanelTitle"
      side="right"
      :class="rightCollapsed ? 'w-10 shrink-0' : 'flex-1 min-w-0'"
    >
      <CodeViewer />
    </CollapsiblePanel>

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
