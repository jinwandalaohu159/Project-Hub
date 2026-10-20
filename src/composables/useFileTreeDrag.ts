import { ref } from "vue";
import type { FileNode } from "./useFileStore";
import { dirname, isChildPath } from "../utils/path";

const DRAG_THRESHOLD_PX = 5;

interface DragOptions {
  /** Project root; dropping on the empty tree area moves an item there. */
  rootPath: () => string | null;
  onStart: (node: FileNode) => void;
  /** Performs the move; dragging state is cleared once it settles. */
  onDrop: (targetDirectoryPath: string, sourcePath: string) => Promise<void>;
  /** Pointer released without a valid drop target. */
  onCancel: () => void;
}

/** Pointer-based drag and drop of file tree rows into folders. */
export function useFileTreeDrag(options: DragOptions) {
  const draggingNode = ref<FileNode | null>(null);
  const dragTargetPath = ref<string | null>(null);
  const dragPosition = ref({ x: 0, y: 0 });
  const isDragging = ref(false);
  const suppressNodeClick = ref(false);
  let dragStartPoint = { x: 0, y: 0 };

  function canDropInto(sourcePath: string, targetPath: string): boolean {
    if (sourcePath === targetPath) return false;
    if (dirname(sourcePath) === targetPath) return false;
    if (isChildPath(targetPath, sourcePath)) return false;
    return true;
  }

  function closestDropTargetPathFromPoint(x: number, y: number): string | null {
    const element = document.elementFromPoint(x, y);
    const row = element?.closest("[data-file-tree-node='true']") as HTMLElement | null;
    if (row?.dataset.fileIsDir === "true" && row.dataset.filePath) return row.dataset.filePath;
    if (row) return null;

    const rootDropArea = element?.closest("[data-file-tree-root-drop='true']");
    const rootPath = options.rootPath();
    if (rootDropArea && rootPath) return rootPath;

    return null;
  }

  function stopDraggingNode() {
    draggingNode.value = null;
    dragTargetPath.value = null;
    isDragging.value = false;
  }

  function releaseSuppressedNodeClick(delay = 80) {
    window.setTimeout(() => {
      suppressNodeClick.value = false;
    }, delay);
  }

  function beginNodePointerDrag(event: MouseEvent, node: FileNode) {
    if (event.button !== 0) return;
    draggingNode.value = node;
    options.onStart(node);
    dragStartPoint = { x: event.clientX, y: event.clientY };
    dragPosition.value = { x: event.clientX, y: event.clientY };

    function onMouseMove(moveEvent: MouseEvent) {
      if (!draggingNode.value) return;
      dragPosition.value = { x: moveEvent.clientX, y: moveEvent.clientY };

      const distance = Math.hypot(
        moveEvent.clientX - dragStartPoint.x,
        moveEvent.clientY - dragStartPoint.y,
      );
      if (!isDragging.value && distance >= DRAG_THRESHOLD_PX) {
        isDragging.value = true;
        suppressNodeClick.value = true;
        document.body.style.cursor = "grabbing";
        document.body.style.userSelect = "none";
      }
      if (!isDragging.value) return;

      const targetPath = closestDropTargetPathFromPoint(moveEvent.clientX, moveEvent.clientY);
      dragTargetPath.value =
        targetPath && canDropInto(draggingNode.value.path, targetPath) ? targetPath : null;
    }

    async function onMouseUp(upEvent: MouseEvent) {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";

      const sourceNode = draggingNode.value;
      const targetPath = isDragging.value ? closestDropTargetPathFromPoint(upEvent.clientX, upEvent.clientY) : null;
      const shouldMove = sourceNode && targetPath && canDropInto(sourceNode.path, targetPath);

      if (shouldMove) {
        await options.onDrop(targetPath, sourceNode.path);
        stopDraggingNode();
      } else {
        stopDraggingNode();
        options.onCancel();
      }
      releaseSuppressedNodeClick();
    }

    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  }

  return {
    draggingNode,
    dragTargetPath,
    dragPosition,
    isDragging,
    suppressNodeClick,
    beginNodePointerDrag,
    stopDraggingNode,
    releaseSuppressedNodeClick,
  };
}
