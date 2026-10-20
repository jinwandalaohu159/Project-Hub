<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { useProjectStore } from "../composables/useProjectStore";
import { useI18n } from "../composables/useI18n";
import { useProjectEvents } from "../composables/useProjectEvents";
import { useAppLog } from "../composables/useAppLog";
import { useConfirm } from "../composables/useConfirm";
import { useToast } from "../composables/useToast";
import { errorText } from "../utils/errors";
import { ref, watch } from "vue";
import { GitMerge, Plus, Trash2 } from "lucide-vue-next";

const { selectedProject, loadBranches, branches } = useProjectStore();
const { t } = useI18n();
const { notifyVersionContextChanged, versionSwitchToken } = useProjectEvents();
const { logOperation } = useAppLog();
const { confirm } = useConfirm();
const toast = useToast();

const showCreate = ref(false);
const showDelete = ref(false);
const showMerge = ref(false);
const newBranchName = ref("");
const switchToNew = ref(true);
const deleteTarget = ref("");
const mergeSource = ref("");
const busy = ref(false);

function currentBranchName() {
  return branches.value.find((branch) => branch.is_current)?.name ?? t('current');
}

function togglePanel(panel: "create" | "delete" | "merge") {
  const next = {
    create: panel === "create" && !showCreate.value,
    delete: panel === "delete" && !showDelete.value,
    merge: panel === "merge" && !showMerge.value,
  };
  showCreate.value = next.create;
  showDelete.value = next.delete;
  showMerge.value = next.merge;
}

async function createBranch() {
  if (!selectedProject.value || !newBranchName.value.trim() || busy.value) return;
  busy.value = true;
  try {
    if (switchToNew.value) {
      const hasChanges = await invoke<boolean>("has_unsaved_changes", {
        repoPath: selectedProject.value.path,
      }).catch(() => false);
      if (hasChanges) {
        toast.warning(t('saveOrDiscardBeforeBlankVersion'));
        return;
      }
    }

    await invoke("create_branch", {
      repoPath: selectedProject.value.path,
      name: newBranchName.value.trim(),
      switch: switchToNew.value,
    });
    await logOperation(selectedProject.value, "version.create", `${selectedProject.value.name}: ${newBranchName.value.trim()}`);
    toast.success(t('createdBranch', { name: newBranchName.value }));
    newBranchName.value = "";
    showCreate.value = false;
    await loadBranches();
    notifyVersionContextChanged();
  } catch (e) {
    toast.error(errorText(e, t));
  } finally {
    busy.value = false;
  }
}

async function requestDeleteBranch() {
  if (!selectedProject.value || !deleteTarget.value || busy.value) return;
  const branchName = deleteTarget.value;
  const ok = await confirm({
    title: t('deleteBranchTitle'),
    message: t('deleteBranchBody', { name: branchName }),
    confirmText: t('delete'),
    danger: true,
  });
  if (!ok) return;
  busy.value = true;
  try {
    await invoke("delete_branch", {
      repoPath: selectedProject.value.path,
      name: branchName,
      force: true,
    });
    await logOperation(selectedProject.value, "version.delete", `${selectedProject.value.name}: ${branchName}`);
    toast.success(t('deletedBranch', { name: branchName }));
    deleteTarget.value = "";
    showDelete.value = false;
    await loadBranches();
    notifyVersionContextChanged();
  } catch (e) {
    toast.error(errorText(e, t));
  } finally {
    busy.value = false;
  }
}

async function mergeBranch() {
  if (!selectedProject.value || !mergeSource.value || busy.value) return;
  const ok = await confirm({
    title: t('merge'),
    message: t('mergeConfirmBody', { name: mergeSource.value, target: currentBranchName() }),
    confirmText: t('merge'),
  });
  if (!ok) return;
  busy.value = true;
  try {
    await invoke("merge_branch", {
      repoPath: selectedProject.value.path,
      sourceBranch: mergeSource.value,
    });
    const conflicts = await invoke<boolean>("has_merge_conflicts", {
      repoPath: selectedProject.value.path,
    });
    if (conflicts) {
      toast.error(t('mergeConflict'));
    } else {
      await logOperation(selectedProject.value, "version.merge", `${selectedProject.value.name}: ${mergeSource.value} -> ${currentBranchName()}`);
      toast.success(t('mergedBranch', { name: mergeSource.value, target: currentBranchName() }));
    }
    mergeSource.value = "";
    showMerge.value = false;
    await loadBranches();
    notifyVersionContextChanged();
  } catch (e) {
    toast.error(errorText(e, t));
    await loadBranches();
    notifyVersionContextChanged();
  } finally {
    busy.value = false;
  }
}

const nonCurrentBranches = () =>
  branches.value.filter((b) => !b.is_current);

watch(versionSwitchToken, () => {
  deleteTarget.value = "";
  mergeSource.value = "";
});
</script>

<template>
  <div>
    <div class="flex items-center justify-center gap-2 pt-0.5 pb-0 flex-nowrap overflow-x-auto">
      <button
        @click="togglePanel('create')"
        class="inline-flex items-center gap-1 text-xs text-muted hover:text-ink hover:bg-surface-raised active:scale-95 rounded-md px-2 py-1 transition-all duration-150 whitespace-nowrap"
      >
        <Plus :size="14" />
        {{ t('newBranch') }}
      </button>
      <button
        @click="togglePanel('delete')"
        class="inline-flex items-center gap-1 text-xs text-muted hover:text-error hover:bg-surface-raised active:scale-95 rounded-md px-2 py-1 transition-all duration-150 whitespace-nowrap"
      >
        <Trash2 :size="14" />
        {{ t('delete') }}
      </button>
      <button
        @click="togglePanel('merge')"
        class="inline-flex items-center gap-1 text-xs text-muted hover:text-ink hover:bg-surface-raised active:scale-95 rounded-md px-2 py-1 transition-all duration-150 whitespace-nowrap"
      >
        <GitMerge :size="14" />
        {{ t('merge') }}
      </button>
    </div>

    <!-- Create branch -->
    <Transition name="rise">
    <div v-if="showCreate" class="py-2 border-t border-border space-y-2">
      <input
        v-model="newBranchName"
        :placeholder="t('branchNamePlaceholder')"
        class="w-full px-3 py-2 text-xs bg-surface-raised border border-border rounded-md text-ink placeholder-text-muted focus:outline-none focus:border-primary"
        @keydown.enter="createBranch"
      />
      <label class="flex items-center gap-2 text-xs text-muted cursor-pointer">
        <input type="checkbox" v-model="switchToNew" class="accent-primary" />
        {{ t('switchToNewBranch') }}
      </label>
      <button
        @click="createBranch"
        :disabled="busy || !newBranchName.trim()"
        class="w-full px-3 py-2 text-xs rounded-md bg-primary text-on-primary hover:bg-primary-hover disabled:opacity-50 transition-colors"
      >{{ t('create') }}</button>
    </div>
    </Transition>

    <!-- Delete branch -->
    <Transition name="rise">
    <div v-if="showDelete" class="py-2 border-t border-border space-y-2">
      <select
        v-model="deleteTarget"
        class="w-full px-3 py-2 text-xs bg-canvas border border-border rounded-md text-ink appearance-none focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary/30"
      >
        <option value="" disabled>{{ t('selectBranchToDelete') }}</option>
        <option v-for="b in nonCurrentBranches()" :key="b.name" :value="b.name">{{ b.name }}</option>
      </select>
      <button
        @click="requestDeleteBranch"
        :disabled="!deleteTarget || busy"
        class="w-full px-3 py-2 text-xs rounded-md bg-danger/20 text-danger hover:bg-danger/30 disabled:opacity-50 transition-colors"
      >{{ t('delete') }}</button>
    </div>
    </Transition>

    <!-- Merge branch -->
    <Transition name="rise">
    <div v-if="showMerge" class="py-2 border-t border-border space-y-2">
      <select
        v-model="mergeSource"
        class="w-full px-3 py-2 text-xs bg-canvas border border-border rounded-md text-ink appearance-none focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary/30"
      >
        <option value="" disabled>{{ t('selectBranchToMerge') }}</option>
        <option v-for="b in nonCurrentBranches()" :key="b.name" :value="b.name">{{ b.name }}</option>
      </select>
      <button
        @click="mergeBranch"
        :disabled="!mergeSource || busy"
        class="w-full px-3 py-2 text-xs rounded-md bg-primary text-on-primary hover:bg-primary-hover disabled:opacity-50 transition-colors"
      >{{ t('mergeIntoCurrent') }}</button>
    </div>
    </Transition>

  </div>
</template>
