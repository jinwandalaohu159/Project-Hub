<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { CloudUpload, Loader2 } from "lucide-vue-next";
import Tooltip from "./Tooltip.vue";
import PublishDialog from "./PublishDialog.vue";
import { useProjectStore } from "../composables/useProjectStore";
import { useProjectEvents } from "../composables/useProjectEvents";
import { useAccount } from "../composables/useAccount";
import { useConfirm } from "../composables/useConfirm";
import { useToast } from "../composables/useToast";
import { useAppLog } from "../composables/useAppLog";
import { useI18n } from "../composables/useI18n";
import { errorText, isErrorCode } from "../utils/errors";
import type { RepoInfo, SyncState } from "../types";

const { selectedProject, hasUnsavedChanges } = useProjectStore();
const { syncRefreshToken, notifySyncChanged } = useProjectEvents();
const { account, ensureLoaded, requestLogin, requireFreshLogin } = useAccount();
const { confirm } = useConfirm();
const toast = useToast();
const { logOperation } = useAppLog();
const { t } = useI18n();

const sync = ref<SyncState | null>(null);
const uploading = ref(false);
const publishOpen = ref(false);

async function loadSync() {
  const project = selectedProject.value;
  if (!project || project.missing || !project.is_git_repo) {
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

watch([selectedProject, syncRefreshToken], loadSync, { immediate: true });

function isGithubUrl(url: string | null | undefined) {
  return !!url && url.toLowerCase().startsWith("https://github.com/");
}

function webUrl(url: string) {
  return url.replace(/\.git$/, "");
}

const pendingCount = computed(() => (sync.value?.remote_url ? sync.value.ahead : 0));

const tooltip = computed(() => {
  if (!sync.value?.remote_url) return t("uploadToCloudFirst");
  if (!sync.value.has_upstream) return t("uploadToCloudNew", { url: webUrl(sync.value.remote_url) });
  if (pendingCount.value === 0) return t("uploadToCloudUpToDate");
  return t("uploadToCloudPending", { count: pendingCount.value });
});

function showUploaded(remoteUrl: string | null | undefined) {
  toast.success(t("uploadedToCloud"), {
    action: isGithubUrl(remoteUrl) ? { label: t("openOnGithub"), run: () => openUrl(webUrl(remoteUrl!)) } : undefined,
  });
}

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

async function upload() {
  const project = selectedProject.value;
  if (!project || uploading.value) return;
  await ensureLoaded();
  await loadSync();

  const remote = sync.value?.remote_url;
  if ((!remote || isGithubUrl(remote)) && !account.value) {
    if (!(await requestLogin())) return;
  }

  if (!remote) {
    publishOpen.value = true;
    return;
  }

  if (await hasUnsavedChanges()) {
    const ok = await confirm({
      title: t("uploadWithUnsavedTitle"),
      message: t("uploadWithUnsavedBody"),
      confirmText: t("uploadAnyway"),
    });
    if (!ok) return;
  }
  await push();
}

async function createRepo(input: { name: string; description: string; private: boolean }) {
  const project = selectedProject.value;
  if (!project) return;
  uploading.value = true;
  try {
    const repo = await invoke<RepoInfo>("publish_to_github", {
      repoPath: project.path,
      name: input.name,
      description: input.description,
      private: input.private,
    });
    publishOpen.value = false;
    showUploaded(repo.clone_url);
    await logOperation(project, "cloud.publish", `${project.name}: ${repo.full_name}`);
  } catch (error) {
    if (isErrorCode(error, "GITHUB_NOT_SIGNED_IN")) {
      publishOpen.value = false;
      void requireFreshLogin();
    } else {
      toast.error(errorText(error, t), { detail: t("uploadFailed") });
    }
  } finally {
    uploading.value = false;
    notifySyncChanged();
  }
}

async function linkRepo(url: string) {
  const project = selectedProject.value;
  if (!project) return;
  try {
    const normalized = await invoke<string>("link_remote", { repoPath: project.path, url });
    publishOpen.value = false;
    await loadSync();
    if (sync.value) sync.value.remote_url = normalized;
    await push();
  } catch (error) {
    toast.error(errorText(error, t));
  }
}
</script>

<template>
  <Tooltip :text="tooltip" position="bottom">
    <button
      class="cloud-btn"
      :class="{ pending: pendingCount > 0 }"
      :disabled="uploading || !selectedProject?.is_git_repo"
      @click="upload"
    >
      <Loader2 v-if="uploading" :size="14" class="spin" />
      <CloudUpload v-else :size="14" />
      <span>{{ uploading ? t('uploading') : t('uploadToCloud') }}</span>
      <Transition name="fade">
        <span v-if="pendingCount > 0 && !uploading" class="cloud-badge">↑{{ pendingCount }}</span>
      </Transition>
    </button>
  </Tooltip>
  <PublishDialog
    :open="publishOpen"
    :busy="uploading"
    :project-name="selectedProject?.name ?? ''"
    @create="createRepo"
    @link="linkRepo"
    @close="publishOpen = false"
  />
</template>

<style scoped>
.cloud-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 26px;
  border: 1px solid var(--color-border);
  border-radius: 7px;
  background: var(--color-surface);
  padding: 0 9px;
  color: var(--color-muted);
  font-size: 12px;
  white-space: nowrap;
}

.cloud-btn:hover:not(:disabled) {
  border-color: color-mix(in srgb, var(--color-primary) 55%, var(--color-border));
  color: var(--color-ink);
}

.cloud-btn.pending {
  color: var(--color-ink);
}

.cloud-btn:disabled {
  opacity: 0.6;
  cursor: default;
}

.cloud-badge {
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-primary) 22%, transparent);
  padding: 0 6px;
  color: var(--color-primary);
  font-size: 11px;
  line-height: 16px;
}
</style>
