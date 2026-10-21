import { onBeforeUnmount } from "vue";

interface WatcherOptions {
  /** Re-reads the visible part of the tree. */
  refresh: () => Promise<void>;
  /** True while something (a drag) must not be disturbed by a refresh. */
  isBusy: () => boolean;
}

/**
 * Keeps the file tree in step with the disk: file-system events trigger a debounced refresh, and a
 * slow poll catches anything the watcher misses (or replaces it where watching is unavailable).
 */
export function useFileTreeWatcher(options: WatcherOptions) {
  let unwatch: (() => void) | null = null;
  let refreshTimer: ReturnType<typeof setTimeout> | null = null;
  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let isRefreshing = false;
  let suppressRefreshUntil = 0;

  function stop() {
    if (refreshTimer) {
      clearTimeout(refreshTimer);
      refreshTimer = null;
    }
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
    if (unwatch) {
      unwatch();
      unwatch = null;
    }
  }

  /** `force` ignores a running suppression (see `suppressFor`), e.g. for a manual refresh. */
  async function refreshNow(force = false) {
    if (options.isBusy()) return;
    if (!force && Date.now() < suppressRefreshUntil) return;
    if (isRefreshing) return;
    isRefreshing = true;
    try {
      await options.refresh();
    } finally {
      isRefreshing = false;
    }
  }

  function scheduleRefresh(delay = 150) {
    if (options.isBusy()) return;
    if (Date.now() < suppressRefreshUntil) return;
    if (refreshTimer) clearTimeout(refreshTimer);
    refreshTimer = setTimeout(() => {
      refreshNow();
    }, delay);
  }

  /** Holds off automatic refreshes, e.g. while the tree already shows a move the disk may lag on. */
  function suppressFor(ms: number) {
    suppressRefreshUntil = Date.now() + ms;
  }

  function startPolling(intervalMs: number) {
    if (pollTimer) clearInterval(pollTimer);
    pollTimer = setInterval(() => {
      if (document.visibilityState === "visible") {
        scheduleRefresh(0);
      }
    }, intervalMs);
  }

  async function start(projectPath: string) {
    stop();
    try {
      const { watch: watchFs } = await import("@tauri-apps/plugin-fs");
      unwatch = await watchFs(
        projectPath,
        () => {
          scheduleRefresh();
        },
        { recursive: true, delayMs: 250 }
      );
      startPolling(30000);
    } catch {
      unwatch = null;
      startPolling(5000);
    }
  }

  onBeforeUnmount(stop);

  return { start, stop, refreshNow, scheduleRefresh, suppressFor };
}
