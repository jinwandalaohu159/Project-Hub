import { computed, effectScope, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useProjectStore } from "./useProjectStore";
import { useProjectEvents } from "./useProjectEvents";
import { useFileStore } from "./useFileStore";
import { useAccount } from "./useAccount";
import { useConfirm } from "./useConfirm";
import { useToast } from "./useToast";
import { useAppLog } from "./useAppLog";
import { useI18n } from "./useI18n";
import { errorText, isErrorCode } from "../utils/errors";
import type { PullOutcome, SyncState } from "../types";

/** The cloud is asked again at most this often per project, unless the user asks. */
const FETCH_MIN_GAP_MS = 60 * 1000;

const sync = ref<SyncState | null>(null);
const uploading = ref(false);
const downloading = ref(false);
/** Both sides have records the other lacks: the user picks which side to keep. */
const divergedOpen = ref(false);
const resolving = ref<"cloud" | "mine" | null>(null);
/** One cloud action at a time: they all move the same branch. */
const busy = computed(() => uploading.value || downloading.value || resolving.value !== null);
const lastFetched = new Map<string, number>();
let started = false;

export function isGithubUrl(url: string | null | undefined) {
  return !!url && url.toLowerCase().startsWith("https://github.com/");
}

export function webUrl(url: string) {
  return url.replace(/\.git$/, "");
}

/**
 * The selected project against its cloud copy: same repository (`remote_url`), same version
 * (`branch`), and how many save records only this computer (`ahead`) or only the cloud
 * (`behind`) has, and the actions that bring the two together. Shared by the upload and download
 * buttons, which only show it.
 */
export function useCloudSync() {
  const { selectedProject, loadBranches } = useProjectStore();
  const { syncRefreshToken, notifySyncChanged, notifyVersionContextChanged } = useProjectEvents();
  const { requireFreshLogin } = useAccount();
  const { confirm } = useConfirm();
  const toast = useToast();
  const { logOperation } = useAppLog();
  const { t } = useI18n();

  function syncedProject() {
    const project = selectedProject.value;
    return project && !project.missing && project.is_git_repo ? project : null;
  }

  /** Compares with the cloud as this computer last saw it; no network. */
  async function refresh() {
    const project = syncedProject();
    if (!project) {
      sync.value = null;
      return;
    }
    try {
      const state = await invoke<SyncState>("get_sync_state", { repoPath: project.path });
      if (project.id === selectedProject.value?.id) sync.value = state;
    } catch {
      sync.value = null;
    }
  }

  /** Asks the cloud what it has now. Quiet: offline or signed out keeps the last known state. */
  async function fetchCloud(force = false) {
    const project = syncedProject();
    // An upload or download running now asks the cloud itself; two at once would contend for git.
    if (!project || !sync.value?.remote_url || busy.value) return;
    if (!force && Date.now() - (lastFetched.get(project.path) ?? 0) < FETCH_MIN_GAP_MS) return;
    lastFetched.set(project.path, Date.now());
    try {
      const state = await invoke<SyncState>("fetch_remote", { repoPath: project.path });
      if (project.id === selectedProject.value?.id) sync.value = state;
    } catch {
      // Not reachable right now; the next project switch or return to the window tries again.
    }
  }

  /** Keeps `sync` current: after any change, and from the cloud on opening a project or the window. */
  function start() {
    if (started) return;
    started = true;
    effectScope(true).run(() => {
      watch(syncRefreshToken, refresh);
      watch(
        selectedProject,
        async () => {
          await refresh();
          await fetchCloud();
        },
        { immediate: true }
      );
    });
    window.addEventListener("focus", () => void fetchCloud());
  }

  function showUploaded(remoteUrl: string | null | undefined) {
    toast.success(t("uploadedToCloud"), {
      action: isGithubUrl(remoteUrl) ? { label: t("openOnGithub"), run: () => openUrl(webUrl(remoteUrl!)) } : undefined,
    });
  }

  /** Uploads the current version. `force` overwrites the cloud's records this computer lacks. */
  async function push(force = false, retried = false): Promise<void> {
    const project = selectedProject.value;
    if (!project) return;
    uploading.value = true;
    try {
      await invoke("push_to_remote", { repoPath: project.path, force });
      showUploaded(sync.value?.remote_url);
      await logOperation(project, "cloud.push", `${project.name}: ${sync.value?.branch ?? ""}${force ? " (force)" : ""}`);
    } catch (error) {
      if (isErrorCode(error, "PUSH_REJECTED") && !force) {
        uploading.value = false;
        const ok = await confirm({
          title: t("pushRejectedTitle"),
          message: t("pushRejectedBody"),
          confirmText: t("pushOverwrite"),
          danger: true,
        });
        if (ok) return await push(true, retried);
      } else if (isErrorCode(error, "GITHUB_NOT_SIGNED_IN") && !retried) {
        uploading.value = false;
        if (await requireFreshLogin()) return await push(force, true);
      } else {
        toast.error(errorText(error, t), { detail: t("uploadFailed") });
      }
    } finally {
      uploading.value = false;
      notifySyncChanged();
    }
  }

  /** Files, versions and history may all have changed. */
  async function reloadProject() {
    const project = selectedProject.value;
    await loadBranches();
    if (project) await useFileStore().loadFileTree(project);
    notifyVersionContextChanged();
  }

  /** Brings the cloud's newer records of the current version down. */
  async function download() {
    const project = selectedProject.value;
    if (!project || busy.value) return;
    downloading.value = true;
    try {
      const outcome = await invoke<PullOutcome>("pull_from_remote", { repoPath: project.path });
      if (outcome.received === 0) {
        toast.success(t("downloadAlreadyLatest"));
      } else {
        await reloadProject();
        toast.success(t("downloadDone", { count: outcome.received }));
        await logOperation(project, "cloud.pull", `${project.name}: ${sync.value?.branch ?? ""} +${outcome.received}`);
      }
    } catch (error) {
      if (isErrorCode(error, "SYNC_DIVERGED")) divergedOpen.value = true;
      else toast.error(errorText(error, t), { detail: t("downloadFailed") });
    } finally {
      downloading.value = false;
      await refresh();
    }
  }

  /** Takes the cloud's records; this computer's own become a separate version. */
  async function useCloudVersion() {
    const project = selectedProject.value;
    if (!project) return;
    resolving.value = "cloud";
    try {
      const backup = await invoke<string>("use_cloud_version", { repoPath: project.path });
      divergedOpen.value = false;
      await reloadProject();
      toast.success(t("downloadUsedCloud"), { detail: t("downloadKeptLocal", { name: backup }), duration: 0 });
      await logOperation(project, "cloud.useCloud", `${project.name}: ${sync.value?.branch ?? ""} -> ${backup}`);
    } catch (error) {
      toast.error(errorText(error, t), { detail: t("downloadFailed") });
    } finally {
      resolving.value = null;
      await refresh();
    }
  }

  /** Keeps this computer's records and overwrites the cloud's. */
  async function useLocalVersion() {
    resolving.value = "mine";
    try {
      await push(true);
      divergedOpen.value = false;
    } finally {
      resolving.value = null;
    }
  }

  return {
    sync,
    uploading,
    downloading,
    divergedOpen,
    resolving,
    busy,
    refresh,
    fetchCloud,
    start,
    push,
    showUploaded,
    download,
    useCloudVersion,
    useLocalVersion,
  };
}
