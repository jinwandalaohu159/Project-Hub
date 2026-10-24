import { effectScope, ref, watch, type EffectScope } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useProjectStore } from "./useProjectStore";
import { useProjectEvents } from "./useProjectEvents";

/** Whether the app is watching the open project; without it, views look again more often. */
const watching = ref(false);
/** The folder asked for last, and the one the app watches now. */
let wanted: string | null = null;
let active: string | null = null;
/** Requests to the app, one after another, so a quick switch cannot arrive out of order. */
let requests: Promise<void> = Promise.resolve();
let started = false;
let scope: EffectScope | null = null;
let unlisten: UnlistenFn | null = null;

/**
 * Follows the open project's folder on disk: the app watches it (src-tauri/src/worktree.rs) and
 * reports each burst of changes once, and returning to the window looks again. Views hear of it
 * through `notifyDiskChanged`.
 */
export function useDiskWatch() {
  const { selectedProject } = useProjectStore();
  const { notifyDiskChanged } = useProjectEvents();

  function follow(path: string | null) {
    wanted = path;
    watching.value = false;
    requests = requests.then(async () => {
      // Switched again meanwhile: only the last folder is worth watching.
      if (wanted !== path) return;
      try {
        if (path) {
          // The app drops the previous watch itself.
          active = null;
          await invoke("watch_worktree", { path });
          active = path;
        } else if (active) {
          const previous = active;
          active = null;
          await invoke("unwatch_worktree", { path: previous });
        }
      } catch {
        // Not watchable here: views fall back to looking on their own.
      }
      if (wanted === path) watching.value = active !== null;
    });
  }

  function onReturn() {
    if (wanted && document.visibilityState === "visible") notifyDiskChanged();
  }

  async function start() {
    if (started) return;
    started = true;
    const stopListening = await listen<string>("worktree-changed", (event) => {
      if (event.payload === active) notifyDiskChanged();
    });
    if (!started) {
      stopListening();
      return;
    }
    unlisten = stopListening;
    scope = effectScope(true);
    scope.run(() => {
      watch(
        () => (selectedProject.value && !selectedProject.value.missing ? selectedProject.value.path : null),
        follow,
        { immediate: true },
      );
    });
    window.addEventListener("focus", onReturn);
    document.addEventListener("visibilitychange", onReturn);
  }

  function stop() {
    if (!started) return;
    started = false;
    scope?.stop();
    scope = null;
    unlisten?.();
    unlisten = null;
    window.removeEventListener("focus", onReturn);
    document.removeEventListener("visibilitychange", onReturn);
    follow(null);
  }

  return { watching, start, stop };
}
