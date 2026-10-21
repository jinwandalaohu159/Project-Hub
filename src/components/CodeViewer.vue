<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount } from "vue";
import type * as Monaco from "monaco-editor";
import { useFileStore } from "../composables/useFileStore";
import { useI18n } from "../composables/useI18n";
import { useProjectEvents } from "../composables/useProjectEvents";
import { useToast } from "../composables/useToast";
import { useConfirm } from "../composables/useConfirm";
import { errorText } from "../utils/errors";
import { languageForPath, loadMonaco } from "../utils/monaco";
import { defineEditorThemes } from "../utils/editorThemes";
import { useCodeTheme } from "../composables/useCodeTheme";
import Tooltip from "./Tooltip.vue";
import { Lock, Pencil, Redo2, Save, Undo2 } from "lucide-vue-next";

type SaveState = "idle" | "saving" | "saved" | "error";

const editorContainer = ref<HTMLElement | null>(null);
let monacoApi: typeof Monaco | null = null;
let editor: Monaco.editor.IStandaloneCodeEditor | null = null;
let disposed = false;
let saveStatusTimer: ReturnType<typeof setTimeout> | null = null;
let lastSavedContent = "";
const isDirty = ref(false);
const isReadOnly = ref(true);
const saveState = ref<SaveState>("idle");
const saveError = ref<string | null>(null);

const { selectedFilePath } = useFileStore();
const { t } = useI18n();
const { activeTheme } = useCodeTheme();
const { workingTreeRefreshToken, notifyWorkingTreeChanged } = useProjectEvents();
const toast = useToast();
const { confirm } = useConfirm();

async function ensureMonaco() {
  monacoApi ??= await loadMonaco();
  return monacoApi;
}

async function loadFile(path: string) {
  try {
    const [{ readTextFile }, monaco] = await Promise.all([
      import("@tauri-apps/plugin-fs"),
      ensureMonaco(),
    ]);
    const content = await readTextFile(path);

    if (editor) {
      const lang = languageForPath(path);
      editor.getModel()?.dispose();
      const model = monaco.editor.createModel(content, lang);
      editor.setModel(model);
      editor.updateOptions({ readOnly: isReadOnly.value });
      lastSavedContent = content;
      isDirty.value = false;
      saveState.value = "idle";
      saveError.value = null;
    }
  } catch {
    if (editor) {
      editor.getModel()?.dispose();
      const monaco = await ensureMonaco();
      const model = monaco.editor.createModel(t('unableToReadFile'), "plaintext");
      editor.setModel(model);
    }
    saveState.value = "error";
    saveError.value = t("unableToReadFile");
  }
}

async function readDisk(path: string): Promise<string | null> {
  const { readTextFile } = await import("@tauri-apps/plugin-fs");
  return readTextFile(path).catch(() => null);
}

/**
 * A download, a restore or switching versions may have rewritten the open file. Unedited, the
 * editor follows the disk; with edits, saving asks before overwriting (see `saveFile`).
 */
async function followDisk() {
  const path = selectedFilePath.value;
  if (!editor || !path || isDirty.value) return;
  const content = await readDisk(path);
  if (content === null || content === lastSavedContent || path !== selectedFilePath.value || isDirty.value) return;
  const view = editor.saveViewState();
  lastSavedContent = content;
  editor.getModel()?.setValue(content);
  if (view) editor.restoreViewState(view);
}

async function saveFile() {
  if (!editor || !selectedFilePath.value || isReadOnly.value) return;
  const path = selectedFilePath.value;
  const onDisk = await readDisk(path);
  if (onDisk !== null && onDisk !== lastSavedContent) {
    const ok = await confirm({
      title: t("fileChangedOnDiskTitle"),
      message: t("fileChangedOnDiskBody"),
      confirmText: t("fileOverwrite"),
      danger: true,
    });
    if (!ok || path !== selectedFilePath.value) return;
  }
  if (saveStatusTimer) clearTimeout(saveStatusTimer);
  saveState.value = "saving";
  saveError.value = null;
  try {
    const { writeTextFile } = await import("@tauri-apps/plugin-fs");
    const content = editor.getValue();
    await writeTextFile(path, content);
    lastSavedContent = content;
    isDirty.value = false;
    saveState.value = "saved";
    notifyWorkingTreeChanged();
    saveStatusTimer = setTimeout(() => {
      if (saveState.value === "saved") saveState.value = "idle";
    }, 2200);
  } catch (error) {
    saveState.value = "error";
    saveError.value = errorText(error, t, "saveFailed");
    toast.error(saveError.value);
    saveStatusTimer = setTimeout(() => {
      if (saveState.value === "error") saveState.value = "idle";
      saveError.value = null;
    }, 6000);
  }
}

function toggleReadOnly() {
  isReadOnly.value = !isReadOnly.value;
  if (editor) {
    editor.updateOptions({ readOnly: isReadOnly.value });
  }
}

function undoEdit() {
  if (!editor || isReadOnly.value) return;
  editor.trigger("toolbar", "undo", null);
  editor.focus();
}

function redoEdit() {
  if (!editor || isReadOnly.value) return;
  editor.trigger("toolbar", "redo", null);
  editor.focus();
}

function statusLabel() {
  if (saveState.value === "saving") return t("savingFile");
  if (saveState.value === "saved") return t("saved");
  if (saveState.value === "error") return t("saveFailed");
  if (isDirty.value) return t("unsaved");
  return isReadOnly.value ? t("readOnly") : t("editing");
}

function statusClass() {
  if (saveState.value === "saving") return "text-muted bg-surface-raised";
  if (saveState.value === "saved") return "text-success bg-success/10";
  if (saveState.value === "error") return "text-error bg-error/10";
  if (isDirty.value) return "text-warning bg-warning/10";
  return isReadOnly.value ? "text-muted bg-surface-raised" : "text-primary bg-primary/10";
}

watch(selectedFilePath, async (path) => {
  if (path) {
    await loadFile(path);
  } else {
    if (editor) {
      editor.getModel()?.dispose();
      const monaco = await ensureMonaco();
      const model = monaco.editor.createModel("", "plaintext");
      editor.setModel(model);
    }
    lastSavedContent = "";
    isDirty.value = false;
    saveState.value = "idle";
    saveError.value = null;
  }
});

watch(workingTreeRefreshToken, () => void followDisk());

watch(activeTheme, (next) => {
  monacoApi?.editor.setTheme(next.id);
});

onMounted(async () => {
  if (!editorContainer.value) return;
  const monaco = await ensureMonaco();
  if (disposed || !editorContainer.value) return;

  defineEditorThemes(monaco);

  editor = monaco.editor.create(editorContainer.value, {
    value: "",
    language: "plaintext",
    theme: activeTheme.value.id,
    readOnly: true,
    minimap: { enabled: false },
    fontSize: 13,
    lineNumbers: "on",
    scrollBeyondLastLine: false,
    wordWrap: "on",
    automaticLayout: true,
    padding: { top: 8 },
  });

  editor.onDidChangeModelContent(() => {
    if (!isReadOnly.value) {
      isDirty.value = editor?.getValue() !== lastSavedContent;
      saveState.value = "idle";
      saveError.value = null;
    }
  });

  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, () => {
    saveFile();
  });
  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyY, () => {
    redoEdit();
  });
  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyMod.Shift | monaco.KeyCode.KeyZ, () => {
    redoEdit();
  });

  if (selectedFilePath.value) {
    await loadFile(selectedFilePath.value);
  }
});

onBeforeUnmount(() => {
  disposed = true;
  if (saveStatusTimer) clearTimeout(saveStatusTimer);
  editor?.getModel()?.dispose();
  editor?.dispose();
});
</script>

<template>
  <main class="h-full bg-canvas flex flex-col">
    <div v-if="selectedFilePath" class="flex items-center justify-between px-3 py-2 border-b border-border bg-surface">
      <div class="flex items-center gap-2 min-w-0">
        <span class="px-2 py-1 rounded-md text-xs font-medium whitespace-nowrap" :class="statusClass()">
          {{ statusLabel() }}
        </span>
        <Transition name="fade">
          <span v-if="saveError" class="text-xs text-error truncate">{{ saveError }}</span>
        </Transition>
      </div>
      <div class="flex items-center gap-2">
        <Tooltip :text="t('undoShortcut')" position="bottom">
          <button
            v-if="!isReadOnly"
            @click="undoEdit"
            class="w-8 h-8 flex items-center justify-center rounded-md text-muted hover:text-ink hover:bg-surface-raised transition-colors"
          >
            <Undo2 :size="15" />
          </button>
        </Tooltip>
        <Tooltip :text="t('redoShortcut')" position="bottom">
          <button
            v-if="!isReadOnly"
            @click="redoEdit"
            class="w-8 h-8 flex items-center justify-center rounded-md text-muted hover:text-ink hover:bg-surface-raised transition-colors"
          >
            <Redo2 :size="15" />
          </button>
        </Tooltip>
        <button
          @click="toggleReadOnly"
          class="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-md transition-colors"
          :class="isReadOnly ? 'text-muted hover:text-ink' : 'text-primary hover:text-primary-hover'"
        >
          <component :is="isReadOnly ? Lock : Pencil" :size="14" />
          {{ isReadOnly ? t('readOnly') : t('editing') }}
        </button>
        <Tooltip :text="t('saveShortcut')" position="bottom">
          <button
            v-if="!isReadOnly"
            @click="saveFile"
            :disabled="saveState === 'saving'"
            class="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-md bg-primary text-on-primary hover:bg-primary-hover disabled:opacity-60 disabled:cursor-not-allowed transition-colors"
          >
            <Save :size="14" />
            {{ saveState === 'saving' ? t('savingFile') : t('save') }}
          </button>
        </Tooltip>
      </div>
    </div>
    <div ref="editorContainer" class="flex-1 min-h-0" />
  </main>
</template>
