/** A download size in megabytes, the unit installers are measured in. */
export function formatBytes(bytes: number): string {
  return `${(Math.max(0, bytes) / 1024 / 1024).toFixed(1)} MB`;
}
