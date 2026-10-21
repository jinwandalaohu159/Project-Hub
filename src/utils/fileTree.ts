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

/**
 * Moves the node at `sourcePath` into the folder `targetDirectoryPath` (or the top level when it is
 * `rootPath`), renaming its paths to start at `newPath`. Returns false when the node or the target
 * folder is not in the tree.
 */
export function moveNode<T extends TreeNode<T> & { is_loaded?: boolean }>(
  nodes: T[],
  rootPath: string | null,
  sourcePath: string,
  targetDirectoryPath: string,
  newPath: string
): boolean {
  const movedNode = removeNode(nodes, sourcePath);
  if (!movedNode) return false;

  updateNodePath(movedNode, sourcePath, newPath);

  if (rootPath === targetDirectoryPath) {
    nodes.push(movedNode);
    sortNodes(nodes);
    return true;
  }

  const targetNode = findNode(nodes, targetDirectoryPath);
  if (!targetNode || !targetNode.is_dir) return false;

  targetNode.children.push(movedNode);
  targetNode.is_loaded = true;
  sortNodes(targetNode.children);
  return true;
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
