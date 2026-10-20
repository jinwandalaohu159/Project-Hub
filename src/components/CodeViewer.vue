<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount } from "vue";
import type * as Monaco from "monaco-editor";
import { useFileStore } from "../composables/useFileStore";
import { useI18n } from "../composables/useI18n";
import { useTheme } from "../composables/useTheme";
import { useProjectEvents } from "../composables/useProjectEvents";
import { useToast } from "../composables/useToast";
import { errorText } from "../utils/errors";
import Tooltip from "./Tooltip.vue";
import { Lock, Pencil, Redo2, Save, Undo2 } from "lucide-vue-next";

type SaveState = "idle" | "saving" | "saved" | "error";

const editorContainer = ref<HTMLElement | null>(null);
let monacoApi: typeof Monaco | null = null;
let monacoPromise: Promise<typeof Monaco> | null = null;
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
const { theme } = useTheme();
const { notifyWorkingTreeChanged } = useProjectEvents();
const toast = useToast();

async function ensureMonaco() {
  if (monacoApi) return monacoApi;
  if (!monacoPromise) {
    monacoPromise = import("monaco-editor");
  }
  monacoApi = await monacoPromise;
  return monacoApi;
}

function getLanguage(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  const langMap: Record<string, string> = {
    ts: "typescript", tsx: "typescript", js: "javascript", jsx: "javascript",
    vue: "html", rs: "rust", py: "python", json: "json", md: "markdown",
    css: "css", html: "html", toml: "ini", yaml: "yaml", yml: "yaml",
    xml: "xml", sql: "sql", sh: "shell", bash: "shell",
  };
  return langMap[ext] ?? "plaintext";
}

function editorThemeName() {
  return theme.value === "light" ? "warm-light" : "warm-dark";
}

function defineEditorThemes(monaco: typeof Monaco) {
  monaco.editor.defineTheme("warm-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "comment", foreground: "6e6961", fontStyle: "italic" },
      { token: "keyword", foreground: "cc785c" },
      { token: "string", foreground: "7eb87a" },
      { token: "number", foreground: "d4a856" },
      { token: "type", foreground: "6a9fd4" },
      { token: "function", foreground: "e8e0d8" },
      { token: "variable", foreground: "e8e0d8" },
      { token: "delimiter", foreground: "8a827a" },
      { token: "tag", foreground: "cc785c" },
      { token: "attribute.name", foreground: "d4a856" },
      { token: "attribute.value", foreground: "7eb87a" },
    ],
    colors: {
      "editor.background": "#1a1614",
      "editor.foreground": "#e8e0d8",
      "editor.lineHighlightBackground": "#2c2825",
      "editor.selectionBackground": "#35313080",
      "editorCursor.foreground": "#cc785c",
      "editorLineNumber.foreground": "#6e6961",
      "editorLineNumber.activeForeground": "#8a827a",
      "editor.inactiveSelectionBackground": "#2c282550",
      "editorGutter.background": "#1a1614",
      "editorGutter.foldingControlForeground": "#8a827a",
      "editorIndentGuide.background": "#2e2a27",
      "editorIndentGuide.activeBackground": "#3a3632",
      "editor.selectionHighlightBackground": "#35313040",
      "editorBracketMatch.background": "#35313060",
      "editorBracketMatch.border": "#8a827a50",
      "editorOverviewRuler.border": "#2e2a27",
      "scrollbarSlider.background": "#8a827a30",
      "scrollbarSlider.hoverBackground": "#8a827a50",
      "scrollbarSlider.activeBackground": "#8a827a70",
    },
  });

  monaco.editor.defineTheme("warm-light", {
    base: "vs",
    inherit: true,
    rules: [
      { token: "comment", foreground: "9a8a7a", fontStyle: "italic" },
      { token: "keyword", foreground: "b86a50" },
      { token: "string", foreground: "5f8f5a" },
      { token: "number", foreground: "a97925" },
      { token: "type", foreground: "5f86b0" },
      { token: "function", foreground: "2f261f" },
      { token: "variable", foreground: "2f261f" },
      { token: "delimiter", foreground: "75695d" },
      { token: "tag", foreground: "b86a50" },
      { token: "attribute.name", foreground: "a97925" },
      { token: "attribute.value", foreground: "5f8f5a" },
    ],
    colors: {
      "editor.background": "#fffaf2",
      "editor.foreground": "#2f261f",
      "editor.lineHighlightBackground": "#efe6da",
      "editor.selectionBackground": "#e7dacb",
      "editorCursor.foreground": "#cc785c",
      "editorLineNumber.foreground": "#9a8a7a",
      "editorLineNumber.activeForeground": "#5f5349",
      "editor.inactiveSelectionBackground": "#efe6da80",
      "editorGutter.background": "#fffaf2",
      "editorGutter.foldingControlForeground": "#75695d",
      "editorIndentGuide.background": "#eadfd3",
      "editorIndentGuide.activeBackground": "#d8cab9",
      "editor.selectionHighlightBackground": "#e7dacb80",
      "editorBracketMatch.background": "#e7dacb80",
      "editorBracketMatch.border": "#75695d50",
      "editorOverviewRuler.border": "#eadfd3",
      "scrollbarSlider.background": "#75695d28",
      "scrollbarSlider.hoverBackground": "#75695d44",
      "scrollbarSlider.activeBackground": "#75695d66",
    },
  });
}

async function loadFile(path: string) {
  try {
    const [{ readTextFile }, monaco] = await Promise.all([
      import("@tauri-apps/plugin-fs"),
      ensureMonaco(),
    ]);
    const content = await readTextFile(path);

    if (editor) {
      const lang = getLanguage(path);
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

async function saveFile() {
  if (!editor || !selectedFilePath.value || isReadOnly.value) return;
  if (saveStatusTimer) clearTimeout(saveStatusTimer);
  saveState.value = "saving";
  saveError.value = null;
  try {
    const { writeTextFile } = await import("@tauri-apps/plugin-fs");
    const content = editor.getValue();
    await writeTextFile(selectedFilePath.value, content);
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

watch(theme, () => {
  monacoApi?.editor.setTheme(editorThemeName());
});

onMounted(async () => {
  if (!editorContainer.value) return;
  const monaco = await ensureMonaco();
  if (disposed || !editorContainer.value) return;

  defineEditorThemes(monaco);

  editor = monaco.editor.create(editorContainer.value, {
    value: "",
    language: "plaintext",
    theme: editorThemeName(),
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
