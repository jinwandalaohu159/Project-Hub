<script setup lang="ts">
import { useFileStore } from "../composables/useFileStore";
import type { FileNode } from "../composables/useFileStore";
import { useProjectStore } from "../composables/useProjectStore";
import { useI18n } from "../composables/useI18n";
import { useProjectEvents } from "../composables/useProjectEvents";
import { useConfirm } from "../composables/useConfirm";
import { useToast } from "../composables/useToast";
import { useFileTreeDrag } from "../composables/useFileTreeDrag";
import { errorText } from "../utils/errors";
import { dirname, isChildPath, replacePathPrefix } from "../utils/path";
import { findNode, removeNode, sortNodes, updateNodePath } from "../utils/fileTree";
import { watch, ref, onBeforeUnmount } from "vue";
import FileTreeNode from "./FileTreeNode.vue";
import Tooltip from "./Tooltip.vue";
import ModalShell from "./ModalShell.vue";
import ContextMenu from "./ContextMenu.vue";
import { FilePlus, FolderPlus, RefreshCw } from "lucide-vue-next";

const {
  fileTree,
  selectedFilePath,
  loadingFileTree,
  fileTreeError,
  loadFileTree,
  loadDirectory,
  refreshFileTree,
  selectFile,
  createFile,
  createDirectory,
  renamePath,
  movePath,
  deletePath,
} = useFileStore();
const { selectedProject } = useProjectStore();
const { t } = useI18n();
const { notifyWorkingTreeChanged } = useProjectEvents();
const { confirm } = useConfirm();
const toast = useToast();
const treeRoot = ref<HTMLElement | null>(null);
const fileTreeBody = ref<HTMLElement | null>(null);
const expandedDirs = ref(new Set<string>());
const activeNodePath = ref<string | null>(null);
const contextMenu = ref<{ x: number; y: number; node: FileNode } | null>(null);
const nameDialogOpen = ref(false);
const nameDialogTitle = ref("");
const nameDialogAction = ref("");
const nameDialogValue = ref("");
let resolveNameDialog: ((value: string | null) => void) | null = null;
let unwatchFileTree: (() => void) | null = null;
let refreshTimer: ReturnType<typeof setTimeout> | null = null;
let pollTimer: ReturnType<typeof setInterval> | null = null;
let isRefreshing = false;
let suppressRefreshUntil = 0;

const {
  draggingNode,
  dragTargetPath,
  dragPosition,
  isDragging,
  suppressNodeClick,
  beginNodePointerDrag,
  releaseSuppressedNodeClick,
} = useFileTreeDrag({
  rootPath: () => selectedProject.value?.path ?? null,
  onStart: activateNode,
  onDrop: dropNodeIntoDirectory,
  onCancel: () => scheduleRefresh(250),
});

function targetDirectory(node?: FileNode): string | null {
  if (node?.is_dir) return node.path;
  if (node) return dirname(node.path);
  if (!selectedProject.value) return null;
  return selectedFilePath.value ? dirname(selectedFilePath.value) : selectedProject.value.path;
}

function moveNodeInMemory(sourcePath: string, targetDirectoryPath: string, newPath: string) {
  const movedNode = removeNode(fileTree.value, sourcePath);
  if (!movedNode) {
    return;
  }

  updateNodePath(movedNode, sourcePath, newPath);

  if (selectedProject.value?.path === targetDirectoryPath) {
    fileTree.value.push(movedNode);
    sortNodes(fileTree.value);
    fileTree.value = [...fileTree.value];
    return;
  }

  const targetNode = findNode(fileTree.value, targetDirectoryPath);
  if (!targetNode || !targetNode.is_dir) return;

  targetNode.children.push(movedNode);
  targetNode.is_loaded = true;
  sortNodes(targetNode.children);
  fileTree.value = [...fileTree.value];
}

function focusTree() {
  requestAnimationFrame(() => treeRoot.value?.focus());
}

function activateNode(node: FileNode) {
  activeNodePath.value = node.path;
  focusTree();
}

function selectFileNode(path: string) {
  activeNodePath.value = path;
  selectFile(path);
  focusTree();
}

function stopWatchingFileTree() {
  if (refreshTimer) {
    clearTimeout(refreshTimer);
    refreshTimer = null;
  }
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
  if (unwatchFileTree) {
    unwatchFileTree();
    unwatchFileTree = null;
  }
}

async function refreshVisibleTree(force = false) {
  if (isDragging.value || draggingNode.value) return;
  if (!force && Date.now() < suppressRefreshUntil) return;
  if (isRefreshing) return;
  isRefreshing = true;
  try {
    await refreshFileTree(expandedDirs.value);
  } finally {
    isRefreshing = false;
  }
}

const REFRESH_SPIN_MS = 650;
const refreshSpins = ref(0);
const manualRefreshing = ref(false);

/** The refresh button: spins once per click and also re-reads the unsaved changes. */
async function manualRefresh() {
  refreshSpins.value += 1;
  if (manualRefreshing.value) return;
  manualRefreshing.value = true;
  const started = Date.now();
  try {
    await refreshVisibleTree(true);
    notifyWorkingTreeChanged();
  } finally {
    const remaining = Math.max(0, REFRESH_SPIN_MS - (Date.now() - started));
    setTimeout(() => (manualRefreshing.value = false), remaining);
  }
}

function scheduleRefresh(delay = 150) {
  if (isDragging.value || draggingNode.value) return;
  if (Date.now() < suppressRefreshUntil) return;
  if (refreshTimer) clearTimeout(refreshTimer);
  refreshTimer = setTimeout(() => {
    refreshVisibleTree();
  }, delay);
}

function startPollingFileTree(intervalMs = 30000) {
  if (pollTimer) clearInterval(pollTimer);
  pollTimer = setInterval(() => {
    if (document.visibilityState === "visible") {
      scheduleRefresh(0);
    }
  }, intervalMs);
}

async function requestName(title: string, defaultValue: string, actionLabel: string): Promise<string | null> {
  closeContextMenu();
  resolveNameDialog?.(null);
  nameDialogTitle.value = title;
  nameDialogAction.value = actionLabel;
  nameDialogValue.value = defaultValue;
  nameDialogOpen.value = true;
  return new Promise((resolve) => {
    resolveNameDialog = resolve;
  });
}

function settleNameDialog(value: string | null) {
  const resolve = resolveNameDialog;
  resolveNameDialog = null;
  nameDialogOpen.value = false;
  resolve?.(value);
  focusTree();
}

function submitNameDialog() {
  settleNameDialog(nameDialogValue.value);
}

function cancelNameDialog() {
  settleNameDialog(null);
}

async function startWatchingFileTree(projectPath: string) {
  stopWatchingFileTree();
  try {
    const { watch: watchFs } = await import("@tauri-apps/plugin-fs");
    unwatchFileTree = await watchFs(
      projectPath,
      () => {
        scheduleRefresh();
      },
      { recursive: true, delayMs: 250 }
    );
    startPollingFileTree(30000);
  } catch {
    unwatchFileTree = null;
    startPollingFileTree(5000);
  }
}

watch(selectedProject, (project) => {
  loadFileTree(project);
  expandedDirs.value.clear();
  activeNodePath.value = null;
  draggingNode.value = null;
  if (project) {
    startWatchingFileTree(project.path);
  } else {
    stopWatchingFileTree();
  }
}, { immediate: true });

onBeforeUnmount(stopWatchingFileTree);

async function toggleDir(path: string) {
  if (expandedDirs.value.has(path)) {
    expandedDirs.value.delete(path);
  } else {
    expandedDirs.value.add(path);
    await loadDirectory(path);
  }
}

async function runFileOperation(operation: () => Promise<void>) {
  let succeeded = false;
  try {
    await operation();
    succeeded = true;
  } catch (error) {
    toast.error(errorText(error, t));
  } finally {
    if (succeeded) {
      notifyWorkingTreeChanged();
    }
    closeContextMenu();
  }
}

async function createNewFile(node?: FileNode) {
  const dir = targetDirectory(node);
  if (!dir) return;
  const name = await requestName(t("fileNamePrompt"), "untitled.txt", t("create"));
  if (name === null) {
    closeContextMenu();
    return;
  }

  await runFileOperation(async () => {
    const newPath = await createFile(dir, name);
    expandedDirs.value.add(dir);
    await refreshVisibleTree();
    selectFile(newPath);
  });
}

async function createNewFolder(node?: FileNode) {
  const dir = targetDirectory(node);
  if (!dir) return;
  const name = await requestName(t("folderNamePrompt"), "new-folder", t("create"));
  if (name === null) {
    closeContextMenu();
    return;
  }

  await runFileOperation(async () => {
    const newPath = await createDirectory(dir, name);
    expandedDirs.value.add(dir);
    expandedDirs.value.add(newPath);
    await refreshVisibleTree();
  });
}

function replaceExpandedPath(oldPath: string, newPath: string) {
  expandedDirs.value = new Set([...expandedDirs.value].map((path) => replacePathPrefix(path, oldPath, newPath)));
}

function removeExpandedPath(deletedPath: string) {
  expandedDirs.value = new Set(
    [...expandedDirs.value].filter((path) => path !== deletedPath && !isChildPath(path, deletedPath))
  );
}

async function renameNode(node: FileNode) {
  activateNode(node);
  const name = await requestName(t("renamePrompt"), node.name, t("rename"));
  if (name === null || name === node.name) {
    closeContextMenu();
    return;
  }

  await runFileOperation(async () => {
    const newPath = await renamePath(node.path, name);
    replaceExpandedPath(node.path, newPath);
    activeNodePath.value = newPath;
    await refreshVisibleTree();
  });
}

function renameActiveNode() {
  const path = activeNodePath.value ?? selectedFilePath.value;
  if (!path) return;
  const node = findNode(fileTree.value, path);
  if (node) {
    renameNode(node);
  }
}

async function requestDeleteNode(node: FileNode) {
  activateNode(node);
  closeContextMenu();
  const ok = await confirm({
    title: t('deleteConfirmTitle'),
    message: t('deleteConfirmBody', { name: node.name }),
    confirmText: t('delete'),
    danger: true,
  });
  if (!ok) {
    focusTree();
    return;
  }

  await runFileOperation(async () => {
    await deletePath(node.path);
    removeNode(fileTree.value, node.path);
    removeExpandedPath(node.path);
    if (activeNodePath.value === node.path || (activeNodePath.value && isChildPath(activeNodePath.value, node.path))) {
      activeNodePath.value = null;
    }
    fileTree.value = [...fileTree.value];
  });
  focusTree();
}

async function dropNodeIntoDirectory(targetDirectoryPath: string, sourcePath: string) {
  await runFileOperation(async () => {
    const newPath = await movePath(sourcePath, targetDirectoryPath);
    suppressRefreshUntil = Date.now() + 12000;
    moveNodeInMemory(sourcePath, targetDirectoryPath, newPath);
    replaceExpandedPath(sourcePath, newPath);
    if (activeNodePath.value) {
      activeNodePath.value = replacePathPrefix(activeNodePath.value, sourcePath, newPath);
    }
    expandedDirs.value.add(targetDirectoryPath);
    draggingNode.value = null;
  });
}

function openNodeContextMenu(event: MouseEvent, node: FileNode) {
  activateNode(node);
  contextMenu.value = { x: event.clientX, y: event.clientY, node };
}

function closeContextMenu() {
  contextMenu.value = null;
}
</script>

<template>
  <aside
    ref="treeRoot"
    class="h-full bg-surface flex flex-col outline-none"
    tabindex="0"
    @click="closeContextMenu"
    @keydown.f2.prevent="renameActiveNode"
  >
    <div class="flex items-center justify-between px-2 py-1.5 border-b border-border-subtle">
      <div class="min-w-0 text-xs text-muted truncate">{{ selectedProject ? selectedProject.name : t('files') }}</div>
      <div class="flex items-center gap-1">
        <Tooltip :text="t('newFile')" position="bottom">
          <button
            class="w-7 h-7 flex items-center justify-center rounded-md text-muted hover:text-ink hover:bg-surface-raised disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            :disabled="!selectedProject"
            @click.stop="createNewFile()"
          >
            <FilePlus :size="15" />
          </button>
        </Tooltip>
        <Tooltip :text="t('newFolder')" position="bottom">
          <button
            class="w-7 h-7 flex items-center justify-center rounded-md text-muted hover:text-ink hover:bg-surface-raised disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            :disabled="!selectedProject"
            @click.stop="createNewFolder()"
          >
            <FolderPlus :size="15" />
          </button>
        </Tooltip>
        <Tooltip :text="t('refreshFiles')" position="bottom">
          <button
            class="w-7 h-7 flex items-center justify-center rounded-md text-muted hover:text-ink hover:bg-surface-raised disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            :disabled="!selectedProject"
            @click.stop="manualRefresh"
          >
            <RefreshCw :key="refreshSpins" :size="14" :class="{ 'spin-once': refreshSpins > 0 }" />
          </button>
        </Tooltip>
      </div>
    </div>
    <div
      ref="fileTreeBody"
      data-file-tree-root-drop="true"
      class="flex-1 overflow-y-auto p-2"
    >
      <div
        v-if="isDragging && selectedProject"
        data-file-tree-root-drop="true"
        class="mb-1 flex h-7 items-center rounded-sm px-2 text-xs transition-colors"
        :class="dragTargetPath === selectedProject.path ? 'bg-primary/10 text-primary outline outline-1 outline-primary/70' : 'text-muted bg-surface-raised/60'"
      >
        {{ t('projectRoot') }}
      </div>
      <div v-if="!selectedProject" class="flex flex-col items-center justify-center py-8">
        <span class="font-serif text-[22px] font-semibold text-muted leading-tight">{{ t('selectProjectToViewFiles') }}</span>
      </div>
      <div v-else-if="loadingFileTree" class="flex flex-col items-center justify-center py-8">
        <span class="text-xs text-muted">{{ t('loadingFiles') }}</span>
      </div>
      <div v-else-if="fileTreeError" class="flex flex-col items-center justify-center py-8 px-3 text-center">
        <span class="font-serif text-[18px] font-semibold text-error leading-tight">{{ t('unableToLoadFiles') }}</span>
        <span class="mt-2 text-xs text-muted break-all">{{ fileTreeError }}</span>
      </div>
      <div v-else-if="fileTree.length === 0" class="flex flex-col items-center justify-center py-8">
        <span class="font-serif text-[22px] font-semibold text-muted leading-tight">{{ t('emptyProject') }}</span>
      </div>
      <FileTreeNode
        v-for="node in fileTree"
        :key="node.path"
        :node="node"
        :depth="0"
        :selected-path="selectedFilePath"
        :active-path="activeNodePath"
        :dragging-path="draggingNode?.path ?? null"
        :drag-target-path="dragTargetPath"
        :suppress-click="suppressNodeClick"
        :expanded-dirs="expandedDirs"
        @toggle-dir="toggleDir"
        @select-file="selectFileNode"
        @node-context-menu="openNodeContextMenu"
        @activate-node="activateNode"
        @node-mouse-down="beginNodePointerDrag"
        @click-suppressed="releaseSuppressedNodeClick(0)"
      />
    </div>
    <ContextMenu :at="contextMenu" @close="closeContextMenu">
      <template v-if="contextMenu">
        <button class="menu-item" @click="createNewFile(contextMenu.node)">{{ t('newFile') }}</button>
        <button class="menu-item" @click="createNewFolder(contextMenu.node)">{{ t('newFolder') }}</button>
        <div class="menu-separator" />
        <button class="menu-item" @click="renameNode(contextMenu.node)">{{ t('rename') }}</button>
        <div class="menu-separator" />
        <button class="menu-item menu-item-danger" @click="requestDeleteNode(contextMenu.node)">{{ t('delete') }}</button>
      </template>
    </ContextMenu>
    <div
      v-if="isDragging && draggingNode"
      class="fixed z-[10002] pointer-events-none rounded-md border border-border bg-surface-overlay px-2 py-1 text-xs text-ink shadow-[0_10px_24px_rgb(0_0_0/0.34)]"
      :style="{ left: `${dragPosition.x + 12}px`, top: `${dragPosition.y + 10}px` }"
    >
      {{ draggingNode.name }}
    </div>
    <ModalShell :open="nameDialogOpen" :title="nameDialogTitle" width="420px" @close="cancelNameDialog">
      <form id="file-name-form" @submit.prevent="submitNameDialog">
        <input v-model="nameDialogValue" class="field" spellcheck="false" />
      </form>
      <template #footer>
        <button type="button" class="btn btn-ghost" @click="cancelNameDialog">{{ t('cancel') }}</button>
        <button type="submit" form="file-name-form" class="btn btn-primary" :disabled="!nameDialogValue.trim()">
          {{ nameDialogAction }}
        </button>
      </template>
    </ModalShell>
  </aside>
</template>
