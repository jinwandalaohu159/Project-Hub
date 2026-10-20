<script setup lang="ts">
import type { FileNode } from "../composables/useFileStore"
import { Folder, FolderOpen, File, Loader2 } from "lucide-vue-next";

const props = defineProps<{
  node: FileNode;
  depth: number;
  selectedPath: string | null;
  activePath: string | null;
  draggingPath: string | null;
  dragTargetPath: string | null;
  suppressClick: boolean;
  expandedDirs: Set<string>;
}>();

const emit = defineEmits<{
  toggleDir: [path: string];
  selectFile: [path: string];
  nodeContextMenu: [event: MouseEvent, node: FileNode];
  activateNode: [node: FileNode];
  nodeMouseDown: [event: MouseEvent, node: FileNode];
  clickSuppressed: [];
}>();

function isExpanded(): boolean {
  return props.expandedDirs.has(props.node.path);
}

function isSelected(): boolean {
  return props.selectedPath === props.node.path;
}

function isActive(): boolean {
  return props.activePath === props.node.path;
}

function canDrop(): boolean {
  if (!props.node.is_dir || !props.draggingPath) return false;
  if (props.node.path === props.draggingPath) return false;
  return !(
    props.node.path.startsWith(`${props.draggingPath}/`) ||
    props.node.path.startsWith(`${props.draggingPath}\\`)
  );
}

function isDragTarget(): boolean {
  return props.dragTargetPath === props.node.path && canDrop();
}

function activateAndOpen() {
  if (props.suppressClick) {
    emit("clickSuppressed");
    return;
  }

  emit("activateNode", props.node);
  if (props.node.is_dir) {
    emit("toggleDir", props.node.path);
  } else {
    emit("selectFile", props.node.path);
  }
}

function fileBadge(): string | null {
  if (props.node.is_dir) return null;
  const ext = props.node.name.split(".").pop()?.toLowerCase() ?? "";
  const badgeMap: Record<string, string> = {
    ts: "TS", tsx: "TX", js: "JS", vue: "VU",
    rs: "RS", py: "PY", json: "{}", md: "MD", css: "CS",
    html: "HT", toml: "TM", yaml: "YL",
  };
  return badgeMap[ext] ?? null;
}
</script>

<template>
  <div>
    <div
      data-file-tree-node="true"
      :data-file-path="node.path"
      :data-file-is-dir="node.is_dir ? 'true' : 'false'"
      class="flex items-center gap-1 py-[3px] px-2 cursor-pointer rounded-sm transition-colors text-[12px] select-none"
      :class="[
        isSelected() ? 'bg-surface-overlay text-ink' : isActive() ? 'bg-surface-raised text-ink' : 'hover:bg-surface-raised text-ink',
        isDragTarget() ? 'outline outline-1 outline-primary/70 bg-primary/10' : ''
      ]"
      :style="{ paddingLeft: `${depth * 12 + 4}px` }"
      @click="activateAndOpen"
      @contextmenu.prevent.stop="emit('nodeContextMenu', $event, node)"
      @mousedown.left="emit('nodeMouseDown', $event, node)"
    >
      <component
        :is="node.is_loading ? Loader2 : node.is_dir ? (isExpanded() ? FolderOpen : Folder) : File"
        :size="14"
        class="shrink-0 text-muted"
        :class="node.is_loading ? 'animate-spin' : ''"
      />
      <span class="truncate">{{ node.name }}</span>
      <span v-if="fileBadge()" class="ml-auto text-[11px] font-mono px-1 rounded-sm bg-surface-raised text-muted leading-4">{{ fileBadge() }}</span>
    </div>
    <template v-if="node.is_dir && isExpanded()">
      <FileTreeNode
        v-for="child in node.children"
        :key="child.path"
        :node="child"
        :depth="depth + 1"
        :selected-path="selectedPath"
        :active-path="activePath"
        :dragging-path="draggingPath"
        :drag-target-path="dragTargetPath"
        :suppress-click="suppressClick"
        :expanded-dirs="expandedDirs"
        @toggle-dir="emit('toggleDir', $event)"
        @select-file="emit('selectFile', $event)"
        @node-context-menu="(event, node) => emit('nodeContextMenu', event, node)"
        @activate-node="emit('activateNode', $event)"
        @node-mouse-down="(event, node) => emit('nodeMouseDown', event, node)"
        @click-suppressed="emit('clickSuppressed')"
      />
    </template>
  </div>
</template>
