<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { useProjectStore } from "../composables/useProjectStore";
import { useI18n } from "../composables/useI18n";
import { watch, ref, onMounted, onBeforeUnmount, computed } from "vue";
import { useProjectEvents } from "../composables/useProjectEvents";
import { useAppLog } from "../composables/useAppLog";
import { useToast } from "../composables/useToast";
import { usePersistentRef, clamp } from "../composables/usePersistentRef";
import { errorText } from "../utils/errors";
import { changeDisplayPath } from "../utils/path";
import type { FileDiff, RepoState, StatusSummary } from "../types";
import { Save } from "lucide-vue-next";
import SaveVersionDialog from "./SaveVersionDialog.vue";
import PanelResizer from "./PanelResizer.vue";
import FileStatusChip from "./FileStatusChip.vue";

const STATUS_DISPLAY_LIMIT = 200;
const STATUS_POLL_INTERVAL_MS = 5000;
const DEFAULT_LIST_HEIGHT = 120;

const { selectedProject, branches, loadBranches } = useProjectStore();
const { t } = useI18n();
const {
  workingTreeRefreshToken,
  notifyVersionSaved,
  notifyVersionContextChanged,
  setWorkingTreeChangeCount,
} = useProjectEvents();
const { logOperation } = useAppLog();
const toast = useToast();
const changes = ref<FileDiff[]>([]);
const totalChanges = ref(0);
const truncatedChanges = ref(false);
const showSaveDialog = ref(false);
const saving = ref(false);
const statusInFlight = ref(false);
const listHeight = usePersistentRef("changes-list-height", DEFAULT_LIST_HEIGHT);
let statusPollTimer: ReturnType<typeof setInterval> | null = null;
let statusRequestId = 0;
let lastRepoState: string | null = null;

const currentBranch = computed(() => branches.value.find((branch) => branch.is_current)?.name ?? "");

function resetStatus() {
  changes.value = [];
  totalChanges.value = 0;
  truncatedChanges.value = false;
  setWorkingTreeChangeCount(0);
}

async function loadStatus(options: { force?: boolean } = {}) {
  if ((statusInFlight.value || saving.value) && !options.force) return;

  const requestId = ++statusRequestId;
  const project = selectedProject.value;
  if (!project || project.missing) {
    resetStatus();
    statusInFlight.value = false;
    return;
  }

  statusInFlight.value = true;
  try {
    const summary = await invoke<StatusSummary>("get_git_status_summary", {
      repoPath: project.path,
      limit: STATUS_DISPLAY_LIMIT,
    });
    if (requestId !== statusRequestId) return;
    changes.value = summary.files;
    totalChanges.value = summary.total;
    truncatedChanges.value = summary.truncated;
    setWorkingTreeChangeCount(summary.total);
  } catch {
    if (requestId !== statusRequestId) return;
    resetStatus();
  } finally {
    if (requestId === statusRequestId) {
      statusInFlight.value = false;
    }
  }
}

/** Notices saves or switches done outside the app (agents via MCP, terminals). */
async function checkRepoState() {
  const project = selectedProject.value;
  if (!project || project.missing || saving.value) return;
  try {
    const state = await invoke<RepoState>("get_repo_state", { repoPath: project.path });
    const fingerprint = `${project.id}|${state.branch}|${state.head}|${state.draft ?? ""}`;
    if (lastRepoState !== null && fingerprint !== lastRepoState) {
      await loadBranches();
      notifyVersionContextChanged();
    }
    lastRepoState = fingerprint;
  } catch {
    // Status polling already surfaces broken repositories.
  }
}

watch(selectedProject, () => {
  lastRepoState = null;
  checkRepoState();
});
watch([selectedProject, workingTreeRefreshToken], () => loadStatus({ force: true }), { immediate: true });

onMounted(() => {
  checkRepoState();
  statusPollTimer = setInterval(() => {
    if (document.visibilityState === "visible") {
      loadStatus();
      checkRepoState();
    }
  }, STATUS_POLL_INTERVAL_MS);
});

onBeforeUnmount(() => {
  if (statusPollTimer) {
    clearInterval(statusPollTimer);
    statusPollTimer = null;
  }
});

async function saveVersion(input: { title: string; description: string }) {
  const project = selectedProject.value;
  if (!project || saving.value) return;

  const title = input.title || t("defaultSaveMessage");
  const message = input.description ? `${title}\n\n${input.description}` : title;
  saving.value = true;
  statusRequestId += 1;
  statusInFlight.value = false;

  try {
    await invoke("git_save_version", {
      repoPath: project.path,
      message,
    });
    showSaveDialog.value = false;
    await loadBranches();
    await logOperation(project, "save.create", `${project.name}: ${title}`);
    notifyVersionSaved();
    toast.success(t("savedVersionToast", { title }));
  } catch (error) {
    toast.error(errorText(error, t, "saveFailed"));
  } finally {
    saving.value = false;
    lastRepoState = null;
    await loadStatus({ force: true });
    checkRepoState();
  }
}

function onListResize(dy: number) {
  listHeight.value = clamp(listHeight.value + dy, 48, 520);
}

</script>

<template>
  <Transition name="rise">
    <div v-if="totalChanges > 0" class="shrink-0 border-t border-border bg-surface">
      <div class="pl-5 pr-4 pt-2.5">
        <div class="flex items-center justify-between mb-1.5">
          <h3 class="text-xs font-semibold text-muted uppercase tracking-wider">{{ t('changes') }}</h3>
          <span class="text-[11px] text-muted">
            {{ t('changesSummary', { shown: changes.length, total: totalChanges }) }}
          </span>
        </div>
        <div class="overflow-y-auto" :style="{ height: `${listHeight}px` }">
          <div v-for="file in changes" :key="file.path" class="flex items-center gap-2 py-1 text-xs">
            <FileStatusChip :status="file.status" />
            <span class="truncate text-muted flex-1" :title="changeDisplayPath(file)">
              <template v-if="file.old_path">{{ file.old_path }} → </template>{{ file.path }}
            </span>
          </div>
          <div v-if="truncatedChanges" class="pt-1 pb-2 text-[11px] text-muted">
            {{ t('changesTruncated', { limit: changes.length, total: totalChanges }) }}
          </div>
        </div>
      </div>
      <PanelResizer direction="vertical" @resize="onListResize" @reset="listHeight = DEFAULT_LIST_HEIGHT" />
      <div class="pl-5 pr-4 py-2.5">
        <button
          class="btn btn-primary w-full"
          :disabled="saving"
          @click="showSaveDialog = true"
        >
          <Save :size="14" />
          {{ t('saveVersion') }}
        </button>
      </div>
      <SaveVersionDialog
        :open="showSaveDialog"
        :saving="saving"
        :changes="changes"
        :total="totalChanges"
        :branch="currentBranch"
        @save="saveVersion"
        @close="showSaveDialog = false"
      />
    </div>
  </Transition>
</template>
