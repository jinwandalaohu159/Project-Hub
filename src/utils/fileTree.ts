import { replacePathPrefix } from "./path";

/** Minimal shape shared by file tree nodes. */
export interface TreeNode<T extends TreeNode<T>> {
  name: string;
  path: string;
  is_dir: boolean;
  children: T[];
}

export function findNode<T extends TreeNode<T>>(nodes: T[], path: string): T | null {
  for (const node of nodes) {
    if (node.path === path) return node;
    const child = findNode(node.children, path);
    if (child) return child;
  }
  return null;
}

/** Detaches the node at `path` from the tree and returns it. */
export function removeNode<T extends TreeNode<T>>(nodes: T[], path: string): T | null {
  const index = nodes.findIndex((node) => node.path === path);
  if (index >= 0) {
    const [node] = nodes.splice(index, 1);
    return node;
  }

  for (const node of nodes) {
    const child = removeNode(node.children, path);
    if (child) return child;
  }
  return null;
}

export function updateNodePath<T extends TreeNode<T>>(node: T, oldPath: string, newPath: string) {
  node.path = replacePathPrefix(node.path, oldPath, newPath);
  for (const child of node.children) {
    updateNodePath(child, oldPath, newPath);
  }
}

/** Folders first, then by name. */
export function sortNodes<T extends TreeNode<T>>(nodes: T[]) {
  nodes.sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
}
