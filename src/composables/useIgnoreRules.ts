import { invoke } from "@tauri-apps/api/core";
import { useI18n } from "./useI18n";
import { useToast } from "./useToast";
import { useAppLog } from "./useAppLog";
import { useProjectEvents } from "./useProjectEvents";
import { errorText } from "../utils/errors";
import type { IgnoreRules, IgnoreUpdate, Project } from "../types";

const HIDDEN_LIST_LIMIT = 200;

/** Reading and changing the rules that keep files out of a project's versions (.gitignore). */
export function useIgnoreRules() {
  const { t } = useI18n();
  const toast = useToast();
  const { logOperation } = useAppLog();
  const { notifyWorkingTreeChanged } = useProjectEvents();

  function load(project: Project) {
    return invoke<IgnoreRules>("get_ignore_rules", { repoPath: project.path, limit: HIDDEN_LIST_LIMIT });
  }

  /** Applies the change and tells the user what it means; null when it failed (already reported). */
  async function update(project: Project, add: string[], remove: string[] = []): Promise<IgnoreUpdate | null> {
    try {
      const result = await invoke<IgnoreUpdate>("update_ignore_rules", { repoPath: project.path, add, remove });
      if (result.added.length === 0 && result.removed.length === 0) {
        toast.info(t("ignoreUnchanged"));
        return result;
      }
      void logOperation(
        project,
        "ignore.update",
        `${project.name}: +${JSON.stringify(result.added)} -${JSON.stringify(result.removed)}`,
      );
      // .gitignore changed and files may have stopped being saved: the change list must show it.
      notifyWorkingTreeChanged();
      const stopped = result.stopped_saving.length;
      toast.success(t("ignoreUpdated"), stopped > 0 ? { detail: t("ignoreStoppedSaving", { count: stopped }) } : undefined);
      return result;
    } catch (error) {
      toast.error(errorText(error, t));
      return null;
    }
  }

  return { load, update };
}
