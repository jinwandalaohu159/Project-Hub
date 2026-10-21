/**
 * The last loaded value per project, so a project can show its last known state at once while a
 * fresh load runs (stale-while-revalidate). Bounded: the least recently used project goes first.
 */
export interface ProjectCache<T> {
  get(projectId: string): T | undefined;
  set(projectId: string, value: T): void;
  clear(): void;
}

export function createProjectCache<T>(limit = 16): ProjectCache<T> {
  const entries = new Map<string, T>();

  return {
    get(projectId) {
      const value = entries.get(projectId);
      if (value !== undefined) {
        // Re-inserting marks it as most recently used.
        entries.delete(projectId);
        entries.set(projectId, value);
      }
      return value;
    },
    set(projectId, value) {
      entries.delete(projectId);
      entries.set(projectId, value);
      while (entries.size > limit) {
        const oldest = entries.keys().next().value;
        if (oldest === undefined) break;
        entries.delete(oldest);
      }
    },
    clear() {
      entries.clear();
    },
  };
}
