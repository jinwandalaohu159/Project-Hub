<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useProjectStore } from './composables/useProjectStore'
import { useFileStore } from './composables/useFileStore'
import { useI18n } from './composables/useI18n'
import { useProjectEvents } from './composables/useProjectEvents'
import { useTheme } from './composables/useTheme'
import { usePersistentRef, clamp } from './composables/usePersistentRef'
import ProjectPanel from './components/ProjectPanel.vue'
import FileTree from './components/FileTree.vue'
import CodeViewer from './components/CodeViewer.vue'
import PanelResizer from './components/PanelResizer.vue'
import CollapsiblePanel from './components/CollapsiblePanel.vue'
import SettingsOverlay from './components/SettingsOverlay.vue'
import DevPanel from './components/DevPanel.vue'
import ToastHost from './components/ToastHost.vue'
import ConfirmHost from './components/ConfirmHost.vue'
import GithubLoginDialog from './components/GithubLoginDialog.vue'
import UpdateBanner from './components/UpdateBanner.vue'
import { useUpdater } from './composables/useUpdater'
import { Plus, Settings, Columns2, Rows2, Bug } from 'lucide-vue-next'
import Tooltip from './components/Tooltip.vue'
import { isTauri } from './utils/env'

// Served from public/; bound dynamically so the bundler does not try to import it.
const logoUrl = '/app-icon.png'
const DEFAULT_LEFT_WIDTH = 300
const DEFAULT_MIDDLE_WIDTH = 280
const DEFAULT_VERTICAL_SPLIT = 66.3
const MIN_PANEL_WIDTH = 160
const MAX_PANEL_WIDTH = 900

const projectStore = useProjectStore()
const fileStore = useFileStore()
const { t } = useI18n()
const { workingTreeChangeCount, notifyVersionContextChanged } = useProjectEvents()
useTheme()
const updater = useUpdater()

const showSettings = ref(false)
const showDevPanel = ref(false)
let unlistenOpenSettings: UnlistenFn | null = null
let unlistenRepoChanged: UnlistenFn | null = null

const leftWidth = usePersistentRef('left-width', DEFAULT_LEFT_WIDTH)
const middleWidth = usePersistentRef('middle-width', DEFAULT_MIDDLE_WIDTH)

const leftCollapsed = usePersistentRef('left-collapsed', false)
const middleCollapsed = usePersistentRef('middle-collapsed', false)
const rightCollapsed = usePersistentRef('right-collapsed', false)

type LeftLayout = 'horizontal' | 'vertical'
const leftLayout = usePersistentRef<LeftLayout>('left-layout', 'horizontal')

const leftCollapsedVertical = ref(false)
const middleCollapsedVertical = ref(false)
const autoCollapsedFilesForChanges = ref(false)
const filePanelWasCollapsedBeforeChanges = ref(false)

const codePanelTitle = computed(() => {
  const path = fileStore.selectedFilePath.value
  if (path) {
    const parts = path.split('/')
    return parts[parts.length - 1] || t('code')
  }
  return t('code')
})

/** Panels may grow until the code view would drop below a usable width. */
function maxPanelWidth(other: number) {
  const available = typeof window === 'undefined' ? MAX_PANEL_WIDTH : window.innerWidth - other - 240
  return Math.max(MIN_PANEL_WIDTH, Math.min(MAX_PANEL_WIDTH, available))
}

function onLeftResize(dx: number) {
  const other = leftLayout.value === 'horizontal' && !middleCollapsed.value ? middleWidth.value : 0
  leftWidth.value = clamp(leftWidth.value + dx, MIN_PANEL_WIDTH, maxPanelWidth(other))
}

function onMiddleResize(dx: number) {
  const other = leftCollapsed.value ? 40 : leftWidth.value
  middleWidth.value = clamp(middleWidth.value + dx, MIN_PANEL_WIDTH, maxPanelWidth(other))
}

const verticalSplit = usePersistentRef('vertical-split', DEFAULT_VERTICAL_SPLIT) // percentage, project panel gets more space
const verticalFileOffsetPx = 10
const verticalContainerRef = ref<HTMLElement | null>(null)
const toolbarButtonClass = 'w-8 h-8 flex items-center justify-center rounded-md text-muted hover:text-ink hover:bg-surface-raised transition-colors'

function onVerticalResize(dy: number) {
  const containerHeight = verticalContainerRef.value?.clientHeight ?? 600
  const deltaPercent = (dy / containerHeight) * 100
  verticalSplit.value = clamp(verticalSplit.value + deltaPercent, 15, 85)
}

function toggleLeftLayout() {
  leftLayout.value = leftLayout.value === 'horizontal' ? 'vertical' : 'horizontal'
}

watch([leftLayout, workingTreeChangeCount], ([layout, changeCount]) => {
  const shouldCollapseFiles = layout === 'vertical' && changeCount > 0

  if (shouldCollapseFiles && !autoCollapsedFilesForChanges.value) {
    filePanelWasCollapsedBeforeChanges.value = middleCollapsedVertical.value
    middleCollapsedVertical.value = true
    autoCollapsedFilesForChanges.value = true
    return
  }

  if (!shouldCollapseFiles && autoCollapsedFilesForChanges.value) {
    middleCollapsedVertical.value = filePanelWasCollapsedBeforeChanges.value
    autoCollapsedFilesForChanges.value = false
  }
})

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
  <UpdateBanner />
  <div class="flex min-h-0 w-full flex-1">
    <!-- Always-visible app rail: global actions stay put whatever panels are collapsed. -->
    <nav class="app-rail">
      <img :src="logoUrl" alt="Project Hub" class="app-rail-logo" draggable="false" />
      <div class="flex-1" />
      <Tooltip :text="t('debugPanel')">
        <button class="app-rail-btn" :class="{ active: showDevPanel }" @click="showDevPanel = !showDevPanel">
          <Bug :size="18" />
        </button>
      </Tooltip>
      <Tooltip :text="t('settings')">
        <button class="app-rail-btn" :class="{ active: showSettings }" @click="showSettings = true">
          <Settings :size="18" />
          <span v-if="updater.available.value" class="app-rail-dot" />
        </button>
      </Tooltip>
    </nav>

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
      <PanelResizer v-if="!leftCollapsed" side="left" @resize="onLeftResize" @reset="leftWidth = DEFAULT_LEFT_WIDTH" />

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
      <PanelResizer v-if="!middleCollapsed" side="left" @resize="onMiddleResize" @reset="middleWidth = DEFAULT_MIDDLE_WIDTH" />
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
          @reset="verticalSplit = DEFAULT_VERTICAL_SPLIT"
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
      <PanelResizer side="left" @resize="onLeftResize" @reset="leftWidth = DEFAULT_LEFT_WIDTH" />
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

    <!-- Dev panel (Ctrl+Shift+D) -->
    <DevPanel v-if="showDevPanel" @close="showDevPanel = false" />

    <ConfirmHost />
    <GithubLoginDialog />
    <ToastHost />
  </div>
  </div>
</template>

<style scoped>
.app-rail {
  display: flex;
  flex: none;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  width: 44px;
  padding: 10px 0 10px;
  border-right: 1px solid var(--color-border);
  background: var(--color-canvas);
}

.app-rail-logo {
  width: 24px;
  height: 24px;
  margin-bottom: 6px;
  border-radius: 6px;
  user-select: none;
}

.app-rail-btn {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  border-radius: 8px;
  color: var(--color-muted);
}

.app-rail-btn:hover {
  background: var(--color-surface-raised);
  color: var(--color-ink);
}

.app-rail-dot {
  position: absolute;
  top: 6px;
  right: 6px;
  width: 7px;
  height: 7px;
  border: 1.5px solid var(--color-canvas);
  border-radius: 50%;
  background: var(--color-primary);
}

.app-rail-btn.active {
  background: var(--color-surface-raised);
  color: var(--color-primary);
}
</style>
