/**
 * The .gitignore rule that hides exactly one file or folder: anchored at the project root ("/"),
 * folders ending in "/", and gitignore's special characters escaped so a name like "[draft].md"
 * matches only itself.
 */
export function ignoreRuleFor(path: string, projectPath: string, isDir: boolean): string | null {
  const normalize = (value: string) => value.replace(/\\/g, "/").replace(/\/+$/, "");
  const root = normalize(projectPath);
  const full = normalize(path);
  if (!full.startsWith(`${root}/`)) return null;
  const relative = full
    .slice(root.length + 1)
    .split("/")
    .map(escapeSegment)
    .join("/");
  return `/${relative}${isDir ? "/" : ""}`;
}

function escapeSegment(segment: string): string {
  let escaped = segment.replace(/[\\*?[]/g, (char) => `\\${char}`);
  if (/^[#!]/.test(escaped)) escaped = `\\${escaped}`;
  // Trailing spaces are dropped by git unless escaped.
  return escaped.replace(/ +$/, (spaces) => spaces.replace(/ /g, "\\ "));
}
