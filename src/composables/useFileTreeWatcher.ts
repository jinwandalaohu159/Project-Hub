import { onBeforeUnmount, watch } from "vue";
import { useDiskWatch } from "./useDiskWatch";
import { useProjectEvents } from "./useProjectEvents";

interface WatcherOptions {
  /** Re-reads the visible part of the tree. */
  refresh: () => Promise<void>;
  /** True while something (a drag) must not be disturbed by a refresh. */
  isBusy: () => boolean;
}

/** How often the tree looks again on its own: rarely while the folder is watched. */
const POLL_WATCHED_MS = 30000;
const POLL_UNWATCHED_MS = 5000;

/**
 * Keeps the file tree in step with the disk: it refreshes when the project's files change (see
 * useDiskWatch), and a poll catches anything missed.
 */
export function useFileTreeWatcher(options: WatcherOptions) {
  const { diskChangeToken } = useProjectEvents();
  const { watching } = useDiskWatch();
  let running = false;
  let refreshTimer: ReturnType<typeof setTimeout> | null = null;
  let pollTimer: ReturnType<typeof setInterval> | null = null;
  let isRefreshing = false;
  let suppressRefreshUntil = 0;

  function clearTimers() {
    if (refreshTimer) {
      clearTimeout(refreshTimer);
      refreshTimer = null;
    }
    if (pollTimer) {
      clearInterval(pollTimer);
      pollTimer = null;
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

  function startPolling() {
    if (pollTimer) clearInterval(pollTimer);
    pollTimer = setInterval(() => {
      if (document.visibilityState === "visible") {
        scheduleRefresh(0);
      }
    }, watching.value ? POLL_WATCHED_MS : POLL_UNWATCHED_MS);
  }

  /** Follows the open project; call again after switching. */
  function start() {
    running = true;
    clearTimers();
    startPolling();
  }

  function stop() {
    running = false;
    clearTimers();
  }

  watch(watching, () => {
    if (running) startPolling();
  });
  watch(diskChangeToken, () => {
    if (running) scheduleRefresh(0);
  });

  onBeforeUnmount(stop);

  return { start, stop, refreshNow, scheduleRefresh, suppressFor };
}
