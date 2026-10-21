import { ref, type Ref } from "vue";
import { createProjectCache, type ProjectCache } from "../utils/projectCache";

/** One kind of per-project data (records, versions, changes, files) as a panel shows it. */
export interface ProjectData<T> {
  /** What is on screen. */
  data: Ref<T>;
  /** Whose data is on screen; views re-key on it so another project's content eases in. */
  projectId: Ref<string | null>;
  /** Puts `projectId`'s last loaded value on screen unless its data already is; true when it is. */
  restore(projectId: string): boolean;
  /** Keeps a freshly loaded value for the next time the project is shown. */
  remember(projectId: string, value: T): void;
  /** Puts a value on screen without keeping it (local edits, errors). */
  show(projectId: string | null, value: T): void;
  /** A fresh value: kept and shown. */
  update(projectId: string, value: T): void;
  /** Nothing on screen, e.g. no project selected. */
  clear(): void;
}

// Keyed by name so a panel that is created again finds what its previous instance loaded.
const caches = new Map<string, ProjectCache<unknown>>();

/**
 * Per-project data with a memory of each project's last loaded value: switching to a project
 * shows that value at once while the fresh load runs (stale-while-revalidate), so panels never
 * blank out in between.
 */
export function useProjectData<T>(name: string, empty: () => T): ProjectData<T> {
  if (!caches.has(name)) caches.set(name, createProjectCache<unknown>());
  const cache = caches.get(name) as ProjectCache<T>;
  const data = ref(empty()) as Ref<T>;
  const projectId = ref<string | null>(null);

  function show(id: string | null, value: T) {
    data.value = value;
    projectId.value = id;
  }

  return {
    data,
    projectId,
    restore(id) {
      if (projectId.value === id) return true;
      const cached = cache.get(id);
      if (cached === undefined) return false;
      show(id, cached);
      return true;
    },
    remember: (id, value) => cache.set(id, value),
    show,
    update(id, value) {
      cache.set(id, value);
      show(id, value);
    },
    clear: () => show(null, empty()),
  };
}
