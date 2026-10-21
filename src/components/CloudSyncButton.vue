<script setup lang="ts">
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { CloudUpload } from "lucide-vue-next";
import CloudActionButton from "./CloudActionButton.vue";
import PublishDialog from "./PublishDialog.vue";
import { useProjectStore } from "../composables/useProjectStore";
import { useProjectEvents } from "../composables/useProjectEvents";
import { useAccount } from "../composables/useAccount";
import { useConfirm } from "../composables/useConfirm";
import { useToast } from "../composables/useToast";
import { useAppLog } from "../composables/useAppLog";
import { useI18n } from "../composables/useI18n";
import { isGithubUrl, useCloudSync, webUrl } from "../composables/useCloudSync";
import { errorText, isErrorCode } from "../utils/errors";
import type { RepoInfo } from "../types";

const { selectedProject, hasUnsavedChanges } = useProjectStore();
const { notifySyncChanged } = useProjectEvents();
const { account, ensureLoaded, requestLogin, requireFreshLogin } = useAccount();
const { confirm } = useConfirm();
const toast = useToast();
const { logOperation } = useAppLog();
const { t } = useI18n();
const { sync, uploading, busy, refresh: loadSync, start, push, showUploaded } = useCloudSync();
start();

const publishOpen = ref(false);

const pendingCount = computed(() => (sync.value?.remote_url ? sync.value.ahead : 0));

const tooltip = computed(() => {
  if (!sync.value?.remote_url) return t("uploadToCloudFirst");
  if (!sync.value.has_upstream) return t("uploadToCloudNew", { url: webUrl(sync.value.remote_url) });
  if (pendingCount.value === 0) return t("uploadToCloudUpToDate");
  return t("uploadToCloudPending", { count: pendingCount.value });
});

async function upload() {
  const project = selectedProject.value;
  if (!project || busy.value) return;
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
  <CloudActionButton
    :icon="CloudUpload"
    :label="uploading ? t('uploading') : t('uploadToCloud')"
    :tooltip="tooltip"
    :loading="uploading"
    :disabled="busy || !selectedProject?.is_git_repo"
    :count="pendingCount"
    @click="upload"
  />
  <PublishDialog
    :open="publishOpen"
    :busy="uploading"
    :project-name="selectedProject?.name ?? ''"
    @create="createRepo"
    @link="linkRepo"
    @close="publishOpen = false"
  />
</template>
