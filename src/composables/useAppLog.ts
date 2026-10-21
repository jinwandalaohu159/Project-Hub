import { invoke } from "@tauri-apps/api/core";
import type { Project } from "../types";
import { isTauri } from "../utils/env";

export interface LogEntry {
  timestamp_ms: number;
  level: string;
  project_id: string;
  project_name: string;
  action: string;
  message: string;
}

export interface LogSettings {
  max_mb: number;
  retention_days: number;
  log_dir: string;
}

export type LogLevel = "info" | "warn" | "error";

/** Log id of Project Hub itself (see logs.rs); its log sits next to the per-project ones. */
export const APP_LOG_ID = "project-hub-app";

/** Best effort: logging never gets in the way of what the user is doing. */
async function writeLog(projectId: string, projectName: string, level: LogLevel, action: string, message: string) {
  if (!isTauri()) return;
  try {
    await invoke("write_app_log", { projectId, projectName, level, action, message });
  } catch {
    // Nowhere left to report a failing log.
  }
}

/** An entry in Project Hub's own log. */
export function logApp(level: LogLevel, action: string, message: string) {
  return writeLog(APP_LOG_ID, "Project Hub", level, action, message);
}

export function useAppLog() {
  /** An entry in a project's operation log. */
  async function logOperation(project: Project | null | undefined, action: string, message: string) {
    if (!project) return;
    await writeLog(project.id, project.name, "info", action, message);
  }

  return { logOperation };
}
