import { ref } from "vue";
import type { Project } from "../types";
import { basename, dirname, isChildPath, joinPath, pathDepth, replacePathPrefix, samePath } from "../utils/path";
import { findNode } from "../utils/fileTree";

export interface FileNode {
  name: string;
  path: string;
  is_dir: boolean;
  children: FileNode[];
  is_loaded: boolean;
  is_loading: boolean;
}

const fileTree = ref<FileNode[]>([]);
const selectedFilePath = ref<string | null>(null);
const currentProject = ref<Project | null>(null);
const loadingFileTree = ref(false);
const fileTreeError = ref<string | null>(null);

const IGNORED_DIRS = new Set([".git", "node_modules", ".DS_Store", "__pycache__", ".vscode", "dist", "target"]);
const KEEP_FILE_NAME = ".project-hub-keep";
const IGNORED_FILES = new Set([KEEP_FILE_NAME]);
const INVALID_WINDOWS_NAME_CHARS = /[\\/:*?"<>|]/;

function validateEntryName(name: string) {
  if (!name.trim()) throw new Error("NAME_EMPTY");
  if (name !== name.trim()) throw new Error("NAME_SPACES");
  if (INVALID_WINDOWS_NAME_CHARS.test(name)) {
    throw new Error("NAME_INVALID_CHARS");
  }
}

export function useFileStore() {
  async function loadFileTree(project: Project | null, options: { showLoading?: boolean } = {}) {
    const showLoading = options.showLoading ?? true;
    if (!project) {
      fileTree.value = [];
      selectedFilePath.value = null;
      currentProject.value = null;
      loadingFileTree.value = false;
      fileTreeError.value = null;
      return;
    }
    currentProject.value = project;
    if (showLoading) {
      loadingFileTree.value = true;
    }
    fileTreeError.value = null;
    try {
      const entries = await readDirEntries(project.path);
      fileTree.value = entries;
    } catch (error) {
      fileTree.value = [];
      fileTreeError.value = error instanceof Error ? error.message : String(error);
    } finally {
      if (showLoading) {
        loadingFileTree.value = false;
      }
    }
  }

  async function readDirEntries(dirPath: string): Promise<FileNode[]> {
    const { readDir: tauriReadDir } = await import("@tauri-apps/plugin-fs");
    const entries = await tauriReadDir(dirPath);
    const nodes: FileNode[] = [];

    const sorted = [...entries].sort((a, b) => {
      if (a.isDirectory !== b.isDirectory) return a.isDirectory ? -1 : 1;
      return a.name.localeCompare(b.name);
    });

    for (const entry of sorted) {
      if (IGNORED_DIRS.has(entry.name)) continue;
      if (!entry.isDirectory && IGNORED_FILES.has(entry.name)) continue;

      const fullPath = `${dirPath}/${entry.name}`;
      const node: FileNode = {
        name: entry.name,
        path: fullPath,
        is_dir: entry.isDirectory,
        children: [],
        is_loaded: !entry.isDirectory,
        is_loading: false,
      };

      nodes.push(node);
    }
    return nodes;
  }

  async function loadDirectory(path: string) {
    const node = findNode(fileTree.value, path);
    if (!node || !node.is_dir || node.is_loading) return;
    node.is_loading = true;
    try {
      await mergeDirectoryEntries(path, node.children);
      node.is_loaded = true;
    } catch {
      node.children = [];
      node.is_loaded = false;
    } finally {
      node.is_loading = false;
    }
  }

  async function refreshFileTree(expandedDirs: Iterable<string> = []) {
    const project = currentProject.value;
    if (!project) return;

    fileTreeError.value = null;
    try {
      await mergeDirectoryEntries(project.path, fileTree.value);
      fileTree.value = [...fileTree.value];
    } catch (error) {
      fileTreeError.value = error instanceof Error ? error.message : String(error);
      return;
    }

    const dirs = [...expandedDirs].sort((a, b) => pathDepth(a) - pathDepth(b));
    for (const dir of dirs) {
      await loadDirectory(dir);
    }
  }

  async function mergeDirectoryEntries(dirPath: string, targetNodes: FileNode[]) {
    const freshNodes = await readDirEntries(dirPath);
    const existingByPath = new Map(targetNodes.map((node) => [node.path, node]));
    const merged = freshNodes.map((freshNode) => {
      const existingNode = existingByPath.get(freshNode.path);
      if (!existingNode) return freshNode;

      existingNode.name = freshNode.name;
      existingNode.is_dir = freshNode.is_dir;
      if (!freshNode.is_dir) {
        existingNode.children = [];
        existingNode.is_loaded = true;
      }
      return existingNode;
    });

    targetNodes.splice(0, targetNodes.length, ...merged);
  }

  function selectFile(path: string) {
    selectedFilePath.value = path;
  }

  function clearSelection() {
    selectedFilePath.value = null;
  }

  async function syncDirectoryMarker(dirPath: string) {
    try {
      const { exists, readDir, remove, writeTextFile } = await import("@tauri-apps/plugin-fs");
      const markerPath = joinPath(dirPath, KEEP_FILE_NAME);
      const entries = await readDir(dirPath);
      const hasVersionableEntry = entries.some((entry) => {
        if (entry.name === KEEP_FILE_NAME) return false;
        if (entry.isDirectory && IGNORED_DIRS.has(entry.name)) return false;
        return true;
      });

      if (!hasVersionableEntry && currentProject.value && samePath(dirPath, currentProject.value.path)) {
        if (await exists(markerPath)) await remove(markerPath);
        return;
      }

      if (hasVersionableEntry) {
        if (await exists(markerPath)) await remove(markerPath);
        return;
      }

      if (!(await exists(markerPath))) {
        await writeTextFile(markerPath, "Project Hub keeps this empty folder in versions.\n", { createNew: true });
      }
    } catch {
      // Saving can still fall back to the backend empty-folder scan when marker maintenance fails.
    }
  }

  async function createFile(parentPath: string, name: string): Promise<string> {
    validateEntryName(name);
    const newPath = joinPath(parentPath, name);
    const { exists, writeTextFile } = await import("@tauri-apps/plugin-fs");
    if (await exists(newPath)) throw new Error("NAME_TAKEN");
    await writeTextFile(newPath, "", { createNew: true });
    await syncDirectoryMarker(parentPath);
    selectedFilePath.value = newPath;
    return newPath;
  }

  async function createDirectory(parentPath: string, name: string): Promise<string> {
    validateEntryName(name);
    const newPath = joinPath(parentPath, name);
    const { exists, mkdir } = await import("@tauri-apps/plugin-fs");
    if (await exists(newPath)) throw new Error("NAME_TAKEN");
    await mkdir(newPath);
    await syncDirectoryMarker(newPath);
    await syncDirectoryMarker(parentPath);
    return newPath;
  }

  async function renamePath(path: string, newName: string): Promise<string> {
    validateEntryName(newName);
    const parentPath = dirname(path);
    const newPath = joinPath(parentPath, newName);
    if (newPath === path) return newPath;

    const { exists, rename } = await import("@tauri-apps/plugin-fs");
    // Windows paths are case-insensitive: "readme.md" -> "README.md" must go through a temporary name.
    const caseOnly = newPath.toLowerCase() === path.toLowerCase();
    if (!caseOnly && (await exists(newPath))) throw new Error("NAME_TAKEN");
    if (caseOnly) {
      const temporary = `${path}.ph-rename-${Date.now()}`;
      await rename(path, temporary);
      await rename(temporary, newPath);
    } else {
      await rename(path, newPath);
    }
    await syncDirectoryMarker(parentPath);

    if (selectedFilePath.value) {
      selectedFilePath.value = replacePathPrefix(selectedFilePath.value, path, newPath);
    }

    return newPath;
  }

  async function movePath(path: string, targetDirectoryPath: string): Promise<string> {
    if (path === targetDirectoryPath) throw new Error("MOVE_INTO_SELF");
    if (isChildPath(targetDirectoryPath, path)) throw new Error("MOVE_INTO_CHILD");

    const name = basename(path);
    const currentParentPath = dirname(path);
    const newPath = joinPath(targetDirectoryPath, name);
    if (currentParentPath === targetDirectoryPath || newPath === path) return path;

    const { exists, rename } = await import("@tauri-apps/plugin-fs");
    if (await exists(newPath)) throw new Error("NAME_TAKEN");
    await rename(path, newPath);
    await syncDirectoryMarker(currentParentPath);
    await syncDirectoryMarker(targetDirectoryPath);

    if (selectedFilePath.value) {
      selectedFilePath.value = replacePathPrefix(selectedFilePath.value, path, newPath);
    }

    return newPath;
  }

  async function deletePath(path: string) {
    const parentPath = dirname(path);
    const { remove } = await import("@tauri-apps/plugin-fs");
    await remove(path, { recursive: true });
    await syncDirectoryMarker(parentPath);

    if (selectedFilePath.value === path) {
      selectedFilePath.value = null;
    } else if (selectedFilePath.value && isChildPath(selectedFilePath.value, path)) {
      selectedFilePath.value = null;
    }
  }

  return {
    fileTree,
    selectedFilePath,
    currentProject,
    loadingFileTree,
    fileTreeError,
    loadFileTree,
    loadDirectory,
    refreshFileTree,
    selectFile,
    clearSelection,
    createFile,
    createDirectory,
    renamePath,
    movePath,
    deletePath,
  };
}
