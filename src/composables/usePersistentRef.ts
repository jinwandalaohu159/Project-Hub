import { ref, watch, type Ref } from "vue";

const PREFIX = "project-hub:";

/** A ref mirrored into localStorage, used for panel sizes and layout choices. */
export function usePersistentRef<T>(key: string, initial: T): Ref<T> {
  let start = initial;
  try {
    const saved = window.localStorage.getItem(PREFIX + key);
    if (saved !== null) start = JSON.parse(saved) as T;
  } catch {
    // Corrupt or unavailable storage falls back to the default.
  }
  const value = ref(start) as Ref<T>;
  let timer: ReturnType<typeof setTimeout> | null = null;
  watch(value, (next) => {
    // Dragging updates sizes every frame; write once the user pauses.
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      try {
        window.localStorage.setItem(PREFIX + key, JSON.stringify(next));
      } catch {
        // Ignore quota or privacy-mode errors.
      }
    }, 200);
  }, { deep: true });
  return value;
}

export function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}
