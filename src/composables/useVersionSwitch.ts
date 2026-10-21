import { ref } from "vue";
import { useProjectStore, type SwitchAction } from "./useProjectStore";
import { useI18n } from "./useI18n";
import { useConfirm } from "./useConfirm";
import { useToast } from "./useToast";
import type { FileDiff } from "../types";

/**
 * Switching the current version: a plain confirmation when the work is saved, otherwise the
 * unsaved-changes dialog that asks what to do with them (`showModal` / `chooseAction`).
 */
export function useVersionSwitch() {
  const { branches, switching, switchBranch, getUnsavedChanges } = useProjectStore();
  const { t } = useI18n();
  const { confirm } = useConfirm();
  const toast = useToast();

  const showModal = ref(false);
  const pendingBranch = ref<string | null>(null);
  const unsavedChanges = ref<FileDiff[]>([]);
  /** The version being switched to, while the switch runs. */
  const switchingTarget = ref<string | null>(null);

  async function requestSwitch(branchName: string) {
    if (switching.value) return;

    const branch = branches.value.find((b) => b.name === branchName);
    if (branch?.is_current) return;

    const changes = await getUnsavedChanges();
    if (changes.length > 0) {
      // The unsaved-changes dialog doubles as the confirmation.
      pendingBranch.value = branchName;
      unsavedChanges.value = changes;
      showModal.value = true;
      return;
    }

    const ok = await confirm({
      title: t('switchVersionTitle'),
      message: t('switchVersionBody', { name: branchName }),
      confirmText: t('switchToVersion'),
    });
    if (!ok) return;
    await runSwitch(branchName);
  }

  async function runSwitch(branchName: string, action?: SwitchAction) {
    switchingTarget.value = branchName;
    const result = await switchBranch(branchName, action);
    if (result.success) {
      toast.success(t('switchedVersion', { name: branchName }), action === 'draft' ? { detail: t('draftKeptHint') } : undefined);
    } else if (result.error === "UNSAVED_CHANGES") {
      pendingBranch.value = branchName;
      unsavedChanges.value = await getUnsavedChanges();
      showModal.value = true;
    } else {
      toast.error(result.error ?? t('unknownError'));
    }
    switchingTarget.value = null;
  }

  async function chooseAction(action: SwitchAction) {
    showModal.value = false;
    const target = pendingBranch.value;
    pendingBranch.value = null;
    if (!target) return;
    if (action === "discard") {
      const ok = await confirm({
        title: t('discardChanges'),
        message: t('discardBeforeSwitchConfirm'),
        confirmText: t('discard'),
        danger: true,
      });
      if (!ok) return;
    }
    await runSwitch(target, action);
    unsavedChanges.value = [];
  }

  function cancel() {
    showModal.value = false;
    pendingBranch.value = null;
  }

  return { showModal, pendingBranch, unsavedChanges, switchingTarget, requestSwitch, chooseAction, cancel };
}
