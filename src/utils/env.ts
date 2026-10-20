/** True inside the Tauri webview; false in plain browsers and unit tests. */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
