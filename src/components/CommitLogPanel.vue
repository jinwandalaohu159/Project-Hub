<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { useProjectStore } from "../composables/useProjectStore";
import { useFileStore } from "../composables/useFileStore";
import { useI18n } from "../composables/useI18n";
import { useProjectEvents } from "../composables/useProjectEvents";
import { useAppLog } from "../composables/useAppLog";
import { useConfirm } from "../composables/useConfirm";
import { useToast } from "../composables/useToast";
import { errorText } from "../utils/errors";
import type { CommitEntry, DeleteRecordOutcome, DraftInfo, RestoreOutcome } from "../types";
import { watch, ref } from "vue";
import { ChevronRight, FilePen, Loader2, Pencil, RotateCcw, Trash2 } from "lucide-vue-next";
import { formatRelativeTime } from "../utils/time";
import RecordEditDialog from "./RecordEditDialog.vue";
import ContextMenu from "./ContextMenu.vue";

const HISTORY_LIMIT = 100;
const DRAFT_KEY = "__draft__";

const { selectedProject, loadBranches, hasUnsavedChanges } = useProjectStore();
const { loadFileTree, clearSelection } = useFileStore();
const { t } = useI18n();
const { commitLogRefreshToken, notifyVersionContextChanged } = useProjectEvents();
const { logOperation } = useAppLog();
const { confirm } = useConfirm();
const toast = useToast();
const commits = ref<CommitEntry[]>([]);
const draft = ref<DraftInfo | null>(null);
const restoringKey = ref<string | null>(null);
const expanded = ref(new Set<string>());
const menu = ref<{ x: number; y: number; commit: CommitEntry } | null>(null);
const editing = ref<CommitEntry | null>(null);
const editBusy = ref(false);

async function loadCommits() {
  const project = selectedProject.value;
  if (!project || project.missing) {
    commits.value = [];
    draft.value = null;
    return;
  }
  const [log, currentDraft] = await Promise.all([
    invoke<CommitEntry[]>("get_log", { repoPath: project.path, count: HISTORY_LIMIT }).catch(() => []),
    invoke<DraftInfo | null>("get_draft", { repoPath: project.path }).catch(() => null),
  ]);
  if (project.id !== selectedProject.value?.id) return;
  commits.value = log;
  draft.value = currentDraft;
}

watch([selectedProject, commitLogRefreshToken], loadCommits, { immediate: true });

// A menu or edit opened for one project must not act on the next one.
watch(selectedProject, () => {
  menu.value = null;
  if (!editBusy.value) editing.value = null;
});

function formatTime(iso: string | null): string {
  return formatRelativeTime(iso, t);
}

function formatExactTime(iso: string | null): string {
  if (!iso) return t('unknown');
  return new Date(iso).toLocaleString();
}

function shortHash(hash: string): string {
  return hash.slice(0, 7);
}

function toggleExpanded(hash: string) {
  const next = new Set(expanded.value);
  if (next.has(hash)) next.delete(hash);
  else next.add(hash);
  expanded.value = next;
}

async function afterRestore(outcome: RestoreOutcome, message: string) {
  const project = selectedProject.value;
  if (!project) return;
  toast.success(message, outcome.draft_saved ? { detail: t('draftKeptHint') } : undefined);
  clearSelection();
  await Promise.all([loadFileTree(project, { showLoading: false }), loadBranches()]);
  notifyVersionContextChanged();
}

/** Shows the switch right away; the full refresh follows in the background. */
function markCurrent(hash: string) {
  commits.value = commits.value.map((commit) => ({ ...commit, is_current: commit.hash === hash }));
}

/** Note shown in the confirmation describing what happens to unsaved work. */
async function unsavedNote(): Promise<string | undefined> {
  if (!(await hasUnsavedChanges())) return undefined;
  return draft.value ? t('restoreReplacesDraftNote') : t('restoreKeepsDraftNote');
}

async function restoreCommit(commit: CommitEntry) {
  const project = selectedProject.value;
  if (!project || restoringKey.value || commit.is_current) return;

  const ok = await confirm({
    title: t('restoreCommitTitle'),
    message: t('restoreCommitBody', { message: commit.message, time: formatExactTime(commit.time) }),
    note: await unsavedNote(),
    confirmText: t('switchToRecord'),
  });
  if (!ok) return;

  restoringKey.value = commit.hash;
  try {
    const outcome = await invoke<RestoreOutcome>("restore_commit", {
      repoPath: project.path,
      commitHash: commit.hash,
    });
    markCurrent(commit.hash);
    restoringKey.value = null;
    void logOperation(project, "save.restore", `${project.name}: ${shortHash(commit.hash)} ${commit.message}`);
    await afterRestore(outcome, t('restoredCommit', { message: commit.message }));
  } catch (error) {
    toast.error(errorText(error, t, "restoreCommitFailed"));
  } finally {
    restoringKey.value = null;
  }
}

async function restoreDraft() {
  const project = selectedProject.value;
  const current = draft.value;
  if (!project || !current || restoringKey.value) return;

  const dirty = await hasUnsavedChanges();
  const ok = await confirm({
    title: t('restoreDraftTitle'),
    message: t('restoreDraftBody', { count: current.files }),
    note: dirty ? t('restoreDraftSwapNote') : undefined,
    confirmText: t('switchToRecord'),
  });
  if (!ok) return;

  restoringKey.value = DRAFT_KEY;
  try {
    const outcome = await invoke<RestoreOutcome>("restore_draft", { repoPath: project.path });
    restoringKey.value = null;
    void logOperation(project, "save.restore-draft", `${project.name}: ${current.files} files`);
    await afterRestore(outcome, t('restoredDraft'));
  } catch (error) {
    toast.error(errorText(error, t, "restoreCommitFailed"));
  } finally {
    restoringKey.value = null;
  }
}

function openMenu(event: MouseEvent, commit: CommitEntry) {
  if (restoringKey.value) return;
  menu.value = { x: event.clientX, y: event.clientY, commit };
}

function closeMenu() {
  menu.value = null;
}

/**
 * The first record cannot go while it is the one in use: the version would be left empty. Only
 * knowable when the whole history fits in the list; otherwise the backend has the final say.
 */
function canDelete(commit: CommitEntry) {
  if (commits.value.length <= 1) return false;
  if (commits.value.length >= HISTORY_LIMIT) return true;
  const oldest = commits.value[commits.value.length - 1];
  return !(commit.is_current && oldest?.hash === commit.hash);
}

function startEdit(commit: CommitEntry) {
  closeMenu();
  editing.value = commit;
}

async function afterHistoryChange() {
  await Promise.all([loadCommits(), loadBranches()]);
  notifyVersionContextChanged();
}

async function saveEdit(input: { title: string; description: string }) {
  const project = selectedProject.value;
  const commit = editing.value;
  if (!project || !commit || editBusy.value) return;
  const message = input.description ? `${input.title}\n\n${input.description}` : input.title;
  editBusy.value = true;
  try {
    await invoke("edit_record", { repoPath: project.path, commitHash: commit.hash, message });
    editing.value = null;
    toast.success(t('recordEdited'));
    void logOperation(project, "save.edit", `${project.name}: ${commit.message} -> ${input.title}`);
    await afterHistoryChange();
  } catch (error) {
    toast.error(errorText(error, t));
  } finally {
    editBusy.value = false;
  }
}

async function deleteRecord(commit: CommitEntry) {
  closeMenu();
  const project = selectedProject.value;
  if (!project || restoringKey.value || !canDelete(commit)) return;
  const notes = [commit.is_current ? t('deleteCurrentRecordNote') : "", t('deleteUploadedNote')].filter(Boolean);
  const ok = await confirm({
    title: t('deleteRecordTitle'),
    message: t('deleteRecordBody', { message: commit.message }),
    note: notes.join("\n"),
    confirmText: t('delete'),
    danger: true,
  });
  if (!ok) return;
  restoringKey.value = commit.hash;
  try {
    const outcome = await invoke<DeleteRecordOutcome>("delete_record", {
      repoPath: project.path,
      commitHash: commit.hash,
    });
    commits.value = commits.value.filter((entry) => entry.hash !== commit.hash);
    restoringKey.value = null;
    toast.success(t('recordDeleted'), outcome.kept_as_unsaved ? { detail: t('recordDeletedKept') } : undefined);
    void logOperation(project, "save.delete", `${project.name}: ${shortHash(commit.hash)} ${commit.message}`);
    await afterHistoryChange();
  } catch (error) {
    toast.error(errorText(error, t));
  } finally {
    restoringKey.value = null;
  }
}

async function discardDraft() {
  const project = selectedProject.value;
  if (!project || !draft.value || restoringKey.value) return;
  const ok = await confirm({
    title: t('discardDraftTitle'),
    message: t('discardDraftBody'),
    confirmText: t('discard'),
    danger: true,
  });
  if (!ok) return;
  try {
    await invoke("discard_draft", { repoPath: project.path });
    draft.value = null;
    toast.success(t('draftDiscarded'));
  } catch (error) {
    toast.error(errorText(error, t));
  }
}
</script>

<template>
  <div v-if="commits.length > 0 || draft" class="flex-1 min-h-0 flex flex-col border-t border-border">
    <div class="shrink-0 pl-5 pr-4 py-1.5 border-b border-border flex items-center justify-between">
      <h3 class="text-xs font-semibold text-muted uppercase tracking-wider">{{ t('history') }}</h3>
      <span class="text-[11px] text-muted">{{ commits.length }}</span>
    </div>
    <div class="relative flex-1 min-h-0 overflow-y-auto">
      <Transition name="rise">
        <div
          v-if="draft"
          class="group draft-row pl-5 pr-4 py-2 border-b border-border/50 cursor-pointer"
          :class="restoringKey && restoringKey !== DRAFT_KEY ? 'opacity-50 pointer-events-none' : ''"
          @click="restoreDraft"
        >
          <div class="flex items-start gap-2">
            <FilePen :size="14" class="mt-0.5 shrink-0 text-warning" />
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <span class="text-xs text-ink font-medium truncate flex-1">{{ t('draftEntryTitle') }}</span>
                <span class="text-xs text-muted shrink-0">{{ formatTime(draft.time) }}</span>
              </div>
              <div class="mt-1 flex items-center gap-2 text-[11px] text-muted">
                <span>{{ t('draftEntryMeta', { count: draft.files, base: shortHash(draft.base) }) }}</span>
                <button
                  class="ml-auto opacity-0 group-hover:opacity-100 rounded px-1.5 text-muted hover:text-error hover:bg-error/10"
                  @click.stop="discardDraft"
                >
                  {{ t('discard') }}
                </button>
              </div>
            </div>
            <Loader2 v-if="restoringKey === DRAFT_KEY" :size="14" class="mt-0.5 shrink-0 spin text-primary" />
            <RotateCcw v-else :size="14" class="mt-0.5 shrink-0 text-muted opacity-0 group-hover:opacity-100 transition-opacity" />
          </div>
        </div>
      </Transition>
      <TransitionGroup name="list" tag="div" class="relative">
        <div
          v-for="commit in commits"
          :key="commit.hash"
          class="group pl-5 pr-4 py-2 border-b border-border/50 transition-colors"
          :class="[
            restoringKey && restoringKey !== commit.hash ? 'opacity-50 pointer-events-none' : '',
            commit.is_current ? 'bg-success/10 border-l-2 border-l-success cursor-default' : 'hover:bg-surface-raised cursor-pointer'
          ]"
          @click="restoreCommit(commit)"
          @contextmenu.prevent="openMenu($event, commit)"
        >
          <div class="flex items-start gap-2">
            <button
              class="mt-0.5 -ml-1 h-4 w-4 shrink-0 flex items-center justify-center rounded text-muted hover:text-ink"
              :class="commit.body ? '' : 'invisible'"
              :title="t('showDetails')"
              @click.stop="toggleExpanded(commit.hash)"
            >
              <ChevronRight :size="13" class="transition-transform duration-200" :class="{ 'rotate-90': expanded.has(commit.hash) }" />
            </button>
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <span class="text-xs text-ink truncate flex-1">{{ commit.message }}</span>
                <span class="text-xs text-muted shrink-0">{{ commit.time ? formatTime(commit.time) : t('unknown') }}</span>
              </div>
              <Transition name="rise">
                <p
                  v-if="commit.body && expanded.has(commit.hash)"
                  class="mt-1 whitespace-pre-line break-words text-[11.5px] leading-5 text-muted"
                >{{ commit.body }}</p>
              </Transition>
              <div class="mt-1 flex items-center gap-2 text-[11px] text-muted">
                <span v-if="commit.is_current" class="shrink-0 rounded bg-success/15 px-1.5 py-0.5 text-success">{{ t('current') }}</span>
                <span class="truncate">{{ commit.author }}</span>
                <span class="shrink-0 font-mono">{{ shortHash(commit.hash) }}</span>
                <span class="truncate">{{ formatExactTime(commit.time) }}</span>
              </div>
            </div>
            <Loader2 v-if="restoringKey === commit.hash" :size="14" class="mt-0.5 shrink-0 spin text-primary" />
            <RotateCcw
              v-else-if="!commit.is_current"
              :size="14"
              class="mt-0.5 shrink-0 text-muted opacity-0 group-hover:opacity-100 transition-opacity"
            />
          </div>
        </div>
      </TransitionGroup>
    </div>

    <ContextMenu :at="menu" :min-width="200" @close="closeMenu">
      <template v-if="menu">
        <button class="menu-item" @click="startEdit(menu.commit)">
          <Pencil :size="14" class="text-muted" />
          {{ t('editRecord') }}
        </button>
        <div class="menu-separator" />
        <button
          class="menu-item menu-item-danger"
          :disabled="!canDelete(menu.commit)"
          :title="canDelete(menu.commit) ? undefined : t('errorRecordUndeletable')"
          @click="deleteRecord(menu.commit)"
        >
          <Trash2 :size="14" />
          {{ t('deleteRecord') }}
        </button>
      </template>
    </ContextMenu>

    <RecordEditDialog
      :open="editing !== null"
      :busy="editBusy"
      :title="editing?.message ?? ''"
      :description="editing?.body ?? ''"
      @save="saveEdit"
      @close="editing = null"
    />
  </div>
</template>

<style scoped>
.draft-row {
  background: color-mix(in srgb, var(--color-warning) 8%, transparent);
  border-left: 2px dashed color-mix(in srgb, var(--color-warning) 70%, transparent);
}

.draft-row:hover {
  background: color-mix(in srgb, var(--color-warning) 14%, transparent);
}
</style>
