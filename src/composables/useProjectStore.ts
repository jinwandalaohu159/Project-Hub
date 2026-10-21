import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { Project, Branch, FileDiff, StatusSummary, RelocatedProject, VersioningOutcome } from "../types";
import { useFileStore } from "./useFileStore";
import { useI18n } from "./useI18n";
import { useProjectEvents } from "./useProjectEvents";
import { useAppLog } from "./useAppLog";
import { useToast } from "./useToast";
import { errorText } from "../utils/errors";
import { useProjectData } from "./useProjectData";

const projects = ref<Project[]>([]);
const selectedProjectId = ref<string | null>(null);
// The selected project's versions: its last known list at once, then the fresh one.
const versions = useProjectData<Branch[]>("versions", () => []);
const branches = versions.data;
const branchesProjectId = versions.projectId;
const loading = ref(false);
const switching = ref(false);

const selectedProject = computed(() =>
  projects.value.find((p) => p.id === selectedProjectId.value) ?? null
);

/** "draft" keeps the changes as the version's unsaved version before switching. */
export type SwitchAction = "save" | "draft" | "discard";

export function useProjectStore() {
  const { t } = useI18n();
  const { notifyVersionSwitched, notifyVersionContextChanged } = useProjectEvents();
  const { logOperation } = useAppLog();
  const toast = useToast();

  async function loadProjects() {
    loading.value = true;
    try {
      projects.value = await invoke<Project[]>("list_projects");
    } finally {
      loading.value = false;
    }
  }

  /** Re-finds projects whose folder was renamed or moved next to its old place. */
  async function autoRelocate() {
    try {
      const relocated = await invoke<RelocatedProject[]>("auto_relocate_projects");
      if (relocated.length === 0) return;
      await loadProjects();
      for (const item of relocated) {
        toast.info(t("projectAutoRelocated", { name: item.name }), { detail: item.new_path });
      }
      if (relocated.some((item) => item.id === selectedProjectId.value)) {
        await reloadSelected();
      }
    } catch {
      // A failed scan just leaves projects marked as missing.
    }
  }

  async function reloadSelected() {
    const project = selectedProject.value;
    const { loadFileTree, clearSelection } = useFileStore();
    clearSelection();
    await loadBranches();
    await loadFileTree(project && !project.missing ? project : null);
    notifyVersionContextChanged();
  }

  async function addProject() {
    if (loading.value) return;

    const selected = await open({ directory: true, multiple: false });
    if (!selected) return;
    const path = selected;

    loading.value = true;
    try {
      // Version management is switched on automatically for every added folder.
      const added = await invoke<{ project: Project; versioning: VersioningOutcome }>("add_project", { path });
      await loadProjects();
      if (added.versioning.initialized) toast.info(t("versioningEnabled", { name: added.project.name }));
      await selectProject(added.project.id);
    } catch (error) {
      toast.error(errorText(error, t));
    } finally {
      loading.value = false;
    }
  }

  async function relocateProject(id: string) {
    const project = projects.value.find((p) => p.id === id);
    const selected = await open({ directory: true, multiple: false, title: t("relocateProjectTitle", { name: project?.name ?? "" }) });
    if (!selected) return false;
    try {
      const updated = await invoke<Project>("relocate_project", { id, path: selected });
      await loadProjects();
      toast.success(t("projectRelocated", { name: updated.name }));
      if (id === selectedProjectId.value) await reloadSelected();
      return true;
    } catch (error) {
      toast.error(errorText(error, t));
      return false;
    }
  }

  async function removeProject(id: string) {
    await invoke("remove_project", { id });
    if (selectedProjectId.value === id) {
      selectedProjectId.value = null;
      versions.clear();
      const { loadFileTree, clearSelection } = useFileStore();
      await loadFileTree(null);
      clearSelection();
    }
    await loadProjects();
  }

  async function selectProject(id: string) {
    selectedProjectId.value = id;
    versions.restore(id);
    const { clearSelection } = useFileStore();
    clearSelection();
    await loadBranches();
  }

  async function loadBranches() {
    const project = selectedProject.value;
    if (!project || project.missing) {
      versions.show(project?.id ?? null, []);
      return;
    }
    try {
      let list = project.is_git_repo ? await invoke<Branch[]>("get_branches", { repoPath: project.path }) : [];
      if (list.length === 0) {
        // No repository or no record yet: set it up rather than showing an empty, unusable list.
        const outcome = await invoke<VersioningOutcome>("ensure_project_versioned", { id: project.id });
        if (outcome.initialized || outcome.initial_version) {
          if (outcome.initialized) await loadProjects();
          list = await invoke<Branch[]>("get_branches", { repoPath: project.path });
          toast.info(t(outcome.initialized ? "versioningEnabled" : "initialVersionCreated", { name: project.name }));
          notifyVersionContextChanged();
        }
      }
      versions.remember(project.id, list);
      if (project.id === selectedProject.value?.id) versions.show(project.id, list);
    } catch (error) {
      if (project.id !== selectedProject.value?.id) return;
      versions.show(project.id, []);
      // Repeated loads merge into one message instead of stacking.
      toast.error(t("loadVersionsFailed", { name: project.name }), { detail: errorText(error, t) });
    }
  }

  async function getUnsavedChanges(): Promise<FileDiff[]> {
    if (!selectedProject.value) return [];
    try {
      const summary = await invoke<StatusSummary>("get_git_status_summary", {
        repoPath: selectedProject.value.path,
        limit: 200,
      });
      return summary.files;
    } catch {
      return [];
    }
  }

  async function hasUnsavedChanges(): Promise<boolean> {
    if (!selectedProject.value) return false;
    try {
      return await invoke<boolean>("has_unsaved_changes", {
        repoPath: selectedProject.value.path,
      });
    } catch {
      return false;
    }
  }

  async function switchBranch(branchName: string, action?: SwitchAction): Promise<{ success: boolean; error?: string }> {
    const project = selectedProject.value;
    if (!project) return { success: false, error: "No project selected" };
    switching.value = true;
    try {
      const hasChanges = await hasUnsavedChanges();

      if (hasChanges && !action) {
        return { success: false, error: "UNSAVED_CHANGES" };
      }

      if (hasChanges && action === "save") {
        await invoke("git_save_version", {
          repoPath: project.path,
          message: t('autoSaveBeforeSwitch', { name: branchName }),
        });
      } else if (hasChanges && action === "draft") {
        await invoke("stash_to_draft", { repoPath: project.path });
      } else if (hasChanges && action === "discard") {
        await invoke("discard_changes", { repoPath: project.path });
      }
      await invoke("switch_branch", {
        repoPath: project.path,
        branchName,
      });

      await loadBranches();
      const { loadFileTree, clearSelection } = useFileStore();
      await loadFileTree(project);
      clearSelection();
      await logOperation(project, "version.switch", `${project.name}: ${branchName}`);
      notifyVersionSwitched();
      return { success: true };
    } catch (e) {
      return { success: false, error: errorText(e, t) };
    } finally {
      switching.value = false;
    }
  }

  return {
    projects,
    selectedProjectId,
    selectedProject,
    branches,
    branchesProjectId,
    loading,
    switching,
    loadProjects,
    autoRelocate,
    addProject,
    relocateProject,
    removeProject,
    selectProject,
    loadBranches,
    switchBranch,
    hasUnsavedChanges,
    getUnsavedChanges,
  };
}
