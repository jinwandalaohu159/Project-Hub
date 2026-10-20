/** True inside the Tauri webview; false in plain browsers and unit tests. */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** True on macOS, where the window keeps its own traffic-light buttons. */
export function isMac(): boolean {
  return typeof navigator !== "undefined" && /Mac/i.test(navigator.platform || navigator.userAgent);
}
