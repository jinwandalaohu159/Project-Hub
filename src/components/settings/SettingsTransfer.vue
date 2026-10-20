<script setup lang="ts">
import { computed, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Download, FolderOpen, Loader2, Upload } from "lucide-vue-next";
import ModalShell from "../ModalShell.vue";
import { useProjectStore } from "../../composables/useProjectStore";
import { useAccount } from "../../composables/useAccount";
import { useToast } from "../../composables/useToast";
import { useI18n, type TranslationKey } from "../../composables/useI18n";
import { errorText } from "../../utils/errors";
import type { ExportReport, ImportPreview, ImportReport, ImportTargetState } from "../../types";

const { loadProjects, autoRelocate } = useProjectStore();
const { refresh: refreshAccount } = useAccount();
const toast = useToast();
const { t } = useI18n();

const includeHistory = ref(true);
const exporting = ref(false);
const importing = ref(false);
const loadingPreview = ref(false);
const packagePath = ref("");
const preview = ref<ImportPreview | null>(null);
const importOpen = ref(false);
const importAuthor = ref(true);

interface Row {
  id: string;
  name: string;
  original: string;
  target: string;
  state: ImportTargetState;
  include: boolean;
  hasHistory: boolean;
  hasDraft: boolean;
  registered: boolean;
}
const rows = ref<Row[]>([]);

const FILTERS = [{ name: "Project Hub", extensions: ["phub"] }];

function stamp() {
  const d = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}${pad(d.getMonth() + 1)}${pad(d.getDate())}`;
}

async function exportAll() {
  const path = await save({ defaultPath: `project-hub-backup-${stamp()}.phub`, filters: FILTERS });
  if (!path) return;
  exporting.value = true;
  try {
    const report = await invoke<ExportReport>("export_data", { path, includeHistory: includeHistory.value });
    if (report.warnings.length > 0) {
      toast.warning(t("exportDoneWithWarnings", { count: report.exported }), { detail: report.warnings.join("\n") });
    } else {
      toast.success(t("exportDone", { count: report.exported }), { detail: path });
    }
  } catch (error) {
    toast.error(errorText(error, t));
  } finally {
    exporting.value = false;
  }
}

async function stateOf(path: string): Promise<ImportTargetState> {
  if (!path.trim()) return "missing";
  return invoke<ImportTargetState>("get_import_target_state", { path }).catch(() => "missing" as const);
}

async function chooseImport() {
  const selected = await open({ multiple: false, filters: FILTERS });
  if (!selected) return;
  loadingPreview.value = true;
  try {
    const result = await invoke<ImportPreview>("preview_import", { path: selected });
    packagePath.value = selected;
    preview.value = result;
    rows.value = await Promise.all(
      result.projects.map(async (project) => ({
        id: project.id,
        name: project.name,
        original: project.original_path,
        target: project.suggested_path,
        state: await stateOf(project.suggested_path),
        include: !project.registered,
        hasHistory: project.has_history,
        hasDraft: project.has_draft,
        registered: project.registered,
      }))
    );
    importAuthor.value = !!(result.author_name || result.author_email);
    importOpen.value = true;
  } catch (error) {
    toast.error(errorText(error, t));
  } finally {
    loadingPreview.value = false;
  }
}

async function pickTarget(row: Row) {
  const base = row.target || row.original;
  const parent = base.replace(/[\\/][^\\/]*$/, "");
  const selected = await open({ directory: true, multiple: false, defaultPath: parent || undefined });
  if (!selected) return;
  row.target = selected;
  row.state = await stateOf(selected);
}

async function refreshState(row: Row) {
  row.state = await stateOf(row.target);
}

function stateKey(row: Row): TranslationKey {
  if (!row.target.trim()) return "importStateChoose";
  if (row.state === "repo") return "importStateRepo";
  if (!row.hasHistory) return row.state === "missing" ? "importStateNoData" : "importStateLinkOnly";
  if (row.state === "files") return "importStateFiles";
  return "importStateRestore";
}

function stateTone(row: Row) {
  const key = stateKey(row);
  if (key === "importStateChoose" || key === "importStateNoData") return "text-warning";
  if (key === "importStateFiles") return "text-info";
  return "text-success";
}

const selectedCount = computed(() => rows.value.filter((row) => row.include).length);
const canImport = computed(
  () => selectedCount.value > 0 && rows.value.every((row) => !row.include || (row.target.trim() && stateKey(row) !== "importStateNoData"))
);

async function runImport() {
  if (!canImport.value || importing.value) return;
  importing.value = true;
  try {
    const items = rows.value.filter((row) => row.include).map((row) => ({ id: row.id, target_path: row.target.trim() }));
    const report = await invoke<ImportReport>("apply_import", {
      path: packagePath.value,
      items,
      importAuthor: importAuthor.value,
    });
    await loadProjects();
    await autoRelocate();
    await refreshAccount().catch(() => undefined);
    if (report.errors.length > 0) {
      toast.warning(t("importDoneWithErrors", { count: report.imported.length }), { detail: report.errors.join("\n"), duration: 0 });
    } else {
      toast.success(t("importDone", { count: report.imported.length }));
    }
    importOpen.value = false;
  } catch (error) {
    toast.error(errorText(error, t));
  } finally {
    importing.value = false;
  }
}
</script>

<template>
  <div class="setting-group">
    <label class="setting-label">{{ t('transferSection') }}</label>
    <div class="setting-card space-y-3">
      <div class="setting-description mt-0">{{ t('transferDescription') }}</div>
      <label class="flex items-center gap-2 text-xs text-muted cursor-pointer select-none">
        <input v-model="includeHistory" type="checkbox" class="accent-primary" />
        {{ t('exportIncludeHistory') }}
      </label>
      <div class="log-actions">
        <button class="secondary-btn action-btn" :disabled="exporting" @click="exportAll">
          <Loader2 v-if="exporting" :size="14" class="spin" />
          <Download v-else :size="14" />
          {{ exporting ? t('exporting') : t('exportData') }}
        </button>
        <button class="secondary-btn action-btn" :disabled="loadingPreview || importing" @click="chooseImport">
          <Loader2 v-if="loadingPreview" :size="14" class="spin" />
          <Upload v-else :size="14" />
          {{ t('importData') }}
        </button>
      </div>
    </div>
  </div>

  <ModalShell
    :open="importOpen"
    :busy="importing"
    :title="t('importTitle')"
    :subtitle="preview ? t('importSubtitle', { time: new Date(preview.exported_at).toLocaleString(), count: rows.length }) : undefined"
    width="640px"
    @close="importOpen = false"
  >
    <div class="space-y-2">
      <div
        v-for="row in rows"
        :key="row.id"
        class="rounded-lg border p-3 transition-colors"
        :class="row.include ? 'border-border bg-canvas' : 'border-border-subtle opacity-60'"
      >
        <div class="flex items-center gap-2">
          <input v-model="row.include" type="checkbox" class="accent-primary" :disabled="importing" />
          <span class="text-sm font-medium text-ink truncate">{{ row.name }}</span>
          <span v-if="row.registered" class="rounded bg-surface-raised px-1.5 text-[11px] text-muted">{{ t('importAlreadyAdded') }}</span>
          <span v-if="row.hasDraft" class="rounded bg-warning/15 px-1.5 text-[11px] text-warning">{{ t('importHasDraft') }}</span>
        </div>
        <div class="mt-1 truncate pl-6 text-[11px] text-text-muted" :title="row.original">{{ t('importOriginalPath') }} {{ row.original }}</div>
        <div class="mt-2 flex gap-2 pl-6">
          <input
            v-model="row.target"
            class="field py-1.5 text-xs"
            :placeholder="t('importChooseFolder')"
            :disabled="!row.include || importing"
            @change="refreshState(row)"
          />
          <button class="btn btn-secondary btn-sm" :disabled="!row.include || importing" @click="pickTarget(row)">
            <FolderOpen :size="13" />{{ t('choose') }}
          </button>
        </div>
        <div class="mt-1.5 pl-6 text-[11px]" :class="stateTone(row)">{{ t(stateKey(row)) }}</div>
      </div>
      <label v-if="preview?.author_name || preview?.author_email" class="flex items-center gap-2 pt-1 text-xs text-muted cursor-pointer">
        <input v-model="importAuthor" type="checkbox" class="accent-primary" :disabled="importing" />
        {{ t('importAuthor', { name: preview?.author_name || preview?.author_email || '' }) }}
      </label>
    </div>
    <template #footer>
      <button class="btn btn-ghost" :disabled="importing" @click="importOpen = false">{{ t('cancel') }}</button>
      <button class="btn btn-primary" :disabled="!canImport || importing" @click="runImport">
        <Loader2 v-if="importing" :size="14" class="spin" />
        {{ importing ? t('importing') : t('importConfirm', { count: selectedCount }) }}
      </button>
    </template>
  </ModalShell>
</template>
