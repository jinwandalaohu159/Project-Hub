import { invoke } from "@tauri-apps/api/core";
import type { Project } from "../types";
import { isTauri } from "../utils/env";

export interface AppLogEntry {
  timestamp_ms: number;
  level: string;
  project_id: string;
  project_name: string;
  action: string;
  message: string;
}

export interface AppLogSettings {
  max_mb: number;
  retention_days: number;
  log_dir: string;
}

export function useAppLog() {
  async function logOperation(project: Project | null | undefined, action: string, message: string) {
    if (!project) return;
    if (!isTauri()) return;
    try {
      await invoke("write_app_log", {
        projectId: project.id,
        projectName: project.name,
        action,
        message,
      });
    } catch {
      // Logging must not block the user operation.
    }
  }

  return { logOperation };
}
