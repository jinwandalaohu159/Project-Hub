import { onMounted, onUnmounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isMac, isTauri } from "../utils/env";

/**
 * Follows the window state the title bar draws: on macOS whether it is in full screen (which
 * hides the traffic lights), elsewhere whether it is maximized (for the restore icon).
 */
export function useWindowState() {
  const maximized = ref(false);
  const fullscreen = ref(false);
  const mac = isMac();
  let syncTimer: ReturnType<typeof setTimeout> | null = null;
  let unlistenResize: (() => void) | null = null;
  let disposed = false;

  async function sync() {
    try {
      const win = getCurrentWindow();
      if (mac) fullscreen.value = await win.isFullscreen();
      else maximized.value = await win.isMaximized();
    } catch {
      // Window state is cosmetic here.
    }
  }

  function onResized() {
    if (syncTimer) clearTimeout(syncTimer);
    if (mac) {
      // macOS plays a snapshot during the full-screen animation and resizes the page at its end, so
      // this is the first moment a change can be seen. Ask once more shortly after, in case the
      // window had not finished updating its state when the resize arrived.
      sync();
      syncTimer = setTimeout(sync, 250);
    } else {
      // Resizing fires many events; only the state after it settles matters.
      syncTimer = setTimeout(sync, 120);
    }
  }

  onMounted(async () => {
    if (!isTauri()) return;
    await sync();
    const unlisten = await getCurrentWindow().onResized(onResized);
    // Unmounted while the listener was being registered.
    if (disposed) unlisten();
    else unlistenResize = unlisten;
  });

  onUnmounted(() => {
    disposed = true;
    if (syncTimer) clearTimeout(syncTimer);
    unlistenResize?.();
    unlistenResize = null;
  });

  return { maximized, fullscreen };
}
