import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Where the update helper is (see src-tauri/src/update/helper.rs). */
export type HelperProgress =
  | { phase: "downloading"; downloaded: number; total: number }
  | { phase: "restarting" }
  | { phase: "done" }
  | { phase: "failed"; error: string };

/** Roughly how long installing and reopening takes; the estimated bar is ~85% full by then. */
const RESTART_ESTIMATE_MS = 6000;
const RESTART_CEILING = 0.94;

/**
 * The update helper's progress for its window: what it reported when the window opened, then
 * every change. Installing has no byte count, so that part of the bar eases toward full on an
 * estimate and fills when the new version opens.
 */
export function useHelperProgress(onReady?: () => void) {
  const version = ref("");
  const progress = ref<HelperProgress>({ phase: "downloading", downloaded: 0, total: 0 });
  const restartFraction = ref(0);
  let restartStarted = 0;
  let frame = 0;
  let unlisten: UnlistenFn | null = null;

  function apply(next: HelperProgress) {
    progress.value = next;
    if (next.phase === "restarting" && !restartStarted) {
      restartStarted = performance.now();
      frame = requestAnimationFrame(tick);
    }
  }

  function tick(now: number) {
    const elapsed = now - restartStarted;
    restartFraction.value = RESTART_CEILING * (1 - Math.exp((-2 * elapsed) / RESTART_ESTIMATE_MS));
    if (progress.value.phase === "restarting") frame = requestAnimationFrame(tick);
  }

  /** 0–1 across the current phase. */
  const fraction = computed(() => {
    const current = progress.value;
    if (current.phase === "downloading") return current.total > 0 ? Math.min(1, current.downloaded / current.total) : 0;
    if (current.phase === "restarting") return restartFraction.value;
    return 1;
  });

  onMounted(async () => {
    unlisten = await listen<HelperProgress>("update-helper-progress", (event) => apply(event.payload));
    const state = await invoke<{ version: string; progress: HelperProgress }>("update_helper_state");
    version.value = state.version;
    apply(state.progress);
    onReady?.();
  });

  onBeforeUnmount(() => {
    unlisten?.();
    cancelAnimationFrame(frame);
  });

  return { version, progress, fraction };
}
