/** Path helpers that accept both "/" and "\\" separators (Windows paths from the file system). */

export function joinPath(dirPath: string, name: string): string {
  return dirPath.endsWith("/") || dirPath.endsWith("\\") ? `${dirPath}${name}` : `${dirPath}/${name}`;
}

export function dirname(path: string): string {
  const slashIndex = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return slashIndex >= 0 ? path.slice(0, slashIndex) : path;
}

export function basename(path: string): string {
  const slashIndex = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return slashIndex >= 0 ? path.slice(slashIndex + 1) : path;
}

export function isChildPath(path: string, parentPath: string): boolean {
  return path.startsWith(`${parentPath}/`) || path.startsWith(`${parentPath}\\`);
}

/** Rewrites `path` when it is `oldPrefix` or lies inside it. */
export function replacePathPrefix(path: string, oldPrefix: string, newPrefix: string): string {
  if (path === oldPrefix) return newPrefix;
  if (isChildPath(path, oldPrefix)) return newPrefix + path.slice(oldPrefix.length);
  return path;
}

export function pathDepth(path: string): number {
  return path.split(/[\\/]/).filter(Boolean).length;
}

export function samePath(a: string, b: string): boolean {
  return a.replace(/\\/g, "/") === b.replace(/\\/g, "/");
}

/** "old → new" for renamed files, otherwise the path itself. */
export function changeDisplayPath(file: { path: string; old_path?: string | null }): string {
  return file.old_path ? `${file.old_path} → ${file.path}` : file.path;
}
