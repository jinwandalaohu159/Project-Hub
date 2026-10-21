<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useProjectStore } from "../composables/useProjectStore";
import { useI18n } from "../composables/useI18n";
import { useConfirm } from "../composables/useConfirm";
import { useToast } from "../composables/useToast";
import { usePersistentRef, clamp } from "../composables/usePersistentRef";
import GitStatusPanel from "./GitStatusPanel.vue";
import VersionList from "./VersionList.vue";
import CommitLogPanel from "./CommitLogPanel.vue";
import CloudSyncButton from "./CloudSyncButton.vue";
import CloudDownloadButton from "./CloudDownloadButton.vue";
import PanelResizer from "./PanelResizer.vue";
import { FolderSearch, Search, TriangleAlert, X } from "lucide-vue-next";
import Tooltip from "./Tooltip.vue";
import type { Project } from "../types";

const DEFAULT_PROJECT_LIST_HEIGHT = 107;

const {
  projects,
  selectedProjectId,
  selectedProject,
  branches,
  loading,
  loadProjects,
  autoRelocate,
  relocateProject,
  removeProject,
  selectProject,
} = useProjectStore();

const { t } = useI18n();
const { confirm } = useConfirm();
const toast = useToast();

const projectQuery = ref("");
const branchQuery = ref("");
const projectListRef = ref<HTMLElement | null>(null);
const projectListHeight = usePersistentRef("project-list-height", DEFAULT_PROJECT_LIST_HEIGHT);

onMounted(async () => {
  await loadProjects();
  await autoRelocate();
});

const currentBranch = computed(() => branches.value.find((branch) => branch.is_current) ?? null);

const selectedProjectLabel = computed(() => {
  const name = selectedProject.value?.name;
  if (!name) return "";
  const chars = Array.from(name);
  return chars.length > 20 ? `${chars.slice(0, 20).join("")}...` : name;
});

const orderedProjects = computed(() => {
  const selectedId = selectedProjectId.value;
  const query = projectQuery.value.trim().toLowerCase();
  const visibleProjects = projects.value.filter((project) => {
    if (!query) return true;
    return (
      project.name.toLowerCase().includes(query) ||
      project.path.toLowerCase().includes(query)
    );
  });

  if (!selectedId) return visibleProjects;
  return [...visibleProjects].sort((a, b) => {
    if (a.id === selectedId) return -1;
    if (b.id === selectedId) return 1;
    return 0;
  });
});

watch(
  selectedProjectId,
  async () => {
    await nextTick();
    projectListRef.value?.scrollTo({ top: 0, behavior: "smooth" });
  }
);

async function confirmRemove(project: Project) {
  const ok = await confirm({
    title: t('removeProject'),
    message: t('removeProjectConfirm', { name: project.name }),
    confirmText: t('remove'),
    danger: true,
  });
  if (!ok) return;
  try {
    await removeProject(project.id);
    toast.success(t('projectRemoved', { name: project.name }));
  } catch (error) {
    toast.error(String(error));
  }
}

function onProjectListResize(dy: number) {
  projectListHeight.value = clamp(projectListHeight.value + dy, 52, 480);
}
</script>

<template>
  <aside class="h-full min-h-0 bg-surface flex flex-col">
    <div class="shrink-0 border-b border-border">
      <div class="pl-5 pr-5 pt-2.5 pb-1.5 flex items-center justify-between gap-2 border-b border-border-subtle">
        <div class="min-w-0 flex items-center gap-2">
          <h3 class="text-xs font-semibold text-muted uppercase tracking-wider shrink-0">{{ t('currentProject') }}</h3>
          <Transition name="fade" mode="out-in">
            <span
              v-if="selectedProjectLabel"
              :key="selectedProjectLabel"
              class="max-w-40 truncate text-[11px] text-panel-title h-5 flex items-center rounded bg-surface-raised px-2"
            >
              {{ selectedProjectLabel }}
            </span>
          </Transition>
        </div>
        <span v-if="projects.length > 0" class="text-[11px] text-muted min-w-5 h-5 flex items-center justify-center rounded bg-surface-raised">{{ projects.length }}</span>
      </div>
      <div v-if="loading && projects.length === 0" class="text-xs text-muted text-center py-4">
        {{ t('loading') }}
      </div>
      <div v-else-if="projects.length === 0" class="flex flex-col items-center justify-center py-8 px-4">
        <span class="font-serif text-[22px] font-semibold text-muted leading-tight">{{ t('noProjects') }}</span>
      </div>
      <template v-else>
        <div class="px-5 py-1.5 border-b border-border-subtle">
          <div class="relative">
            <Search :size="14" class="absolute left-3 top-1/2 -translate-y-1/2 text-muted pointer-events-none" />
            <input
              v-model="projectQuery"
              class="w-full h-8 pl-8 pr-8 text-xs bg-canvas border border-border rounded-md text-ink placeholder-text-muted focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary/30 transition-colors"
              :placeholder="t('searchProjects')"
            />
            <button
              v-if="projectQuery"
              class="absolute right-2 top-1/2 -translate-y-1/2 w-5 h-5 flex items-center justify-center rounded text-muted hover:text-ink hover:bg-surface-raised transition-colors"
              @click="projectQuery = ''"
            >
              <X :size="13" />
            </button>
          </div>
        </div>
        <div ref="projectListRef" class="overflow-y-auto pt-[2.5px] pb-1" :style="{ height: `${projectListHeight}px` }">
          <div v-if="orderedProjects.length === 0" class="flex flex-col items-center justify-center py-4 px-4">
            <span class="font-serif text-[17px] font-semibold text-muted">{{ t('noProjectMatches') }}</span>
          </div>
          <TransitionGroup name="list" tag="div" class="relative">
            <div
              v-for="project in orderedProjects"
              :key="project.id"
              class="group ml-4 mr-4 px-3 py-1.5 cursor-pointer rounded-md transition-colors"
              :class="
                selectedProjectId === project.id
                  ? 'bg-surface-raised'
                  : 'hover:bg-surface-raised'
              "
              @click="selectProject(project.id)"
            >
              <div class="flex items-center justify-between gap-2">
                <span
                  class="text-sm truncate"
                  :class="[
                    selectedProjectId === project.id ? 'text-ink font-medium' : 'text-muted',
                    project.missing ? 'line-through decoration-warning/60' : '',
                  ]"
                >{{ project.name }}</span>
                <div class="flex items-center gap-0.5 shrink-0">
                  <Tooltip v-if="project.missing" :text="t('relocateProject')">
                    <button
                      @click.stop="relocateProject(project.id)"
                      class="w-7 h-7 flex items-center justify-center rounded-md text-warning hover:bg-surface-overlay"
                    >
                      <FolderSearch :size="15" />
                    </button>
                  </Tooltip>
                  <Tooltip :text="t('removeProject')">
                    <button
                      @click.stop="confirmRemove(project)"
                      class="opacity-0 group-hover:opacity-100 w-7 h-7 flex items-center justify-center rounded-md text-muted hover:text-danger hover:bg-surface-overlay transition-all"
                    >
                      <X :size="15" />
                    </button>
                  </Tooltip>
                </div>
              </div>
              <div class="text-xs text-muted truncate mt-0.5">{{ project.path }}</div>
              <span v-if="project.missing" class="inline-flex items-center gap-1 text-xs text-warning">
                <TriangleAlert :size="12" />{{ t('projectMissing') }}
              </span>
              <span v-else-if="!project.is_git_repo" class="text-xs text-warning">{{ t('noGit') }}</span>
            </div>
          </TransitionGroup>
        </div>
        <PanelResizer direction="vertical" @resize="onProjectListResize" @reset="projectListHeight = DEFAULT_PROJECT_LIST_HEIGHT" />
      </template>
    </div>

    <Transition name="fade" mode="out-in">
      <div v-if="selectedProject?.missing" key="missing" class="flex-1 flex flex-col items-center justify-center gap-3 px-6 text-center">
        <TriangleAlert :size="26" class="text-warning" />
        <div>
          <div class="font-serif text-[20px] font-semibold text-ink leading-tight">{{ t('projectMissingTitle') }}</div>
          <p class="mt-2 text-xs leading-5 text-muted break-all">{{ t('projectMissingBody', { path: selectedProject.path }) }}</p>
        </div>
        <div class="flex gap-2">
          <button class="btn btn-primary btn-sm" @click="relocateProject(selectedProject.id)">
            <FolderSearch :size="14" />{{ t('relocateProject') }}
          </button>
          <button class="btn btn-ghost btn-sm" @click="confirmRemove(selectedProject)">{{ t('removeProject') }}</button>
        </div>
      </div>

      <!-- One stable key: switching projects updates the panels in place (each eases its own content
           in) instead of fading the whole area out and rebuilding it empty. -->
      <div v-else-if="selectedProjectId" key="project" class="flex-1 min-h-0 overflow-hidden flex flex-col">
        <div class="shrink-0 pl-5 pr-4 pt-2 pb-2 border-b border-border bg-canvas">
          <div class="flex items-start justify-between gap-3">
            <div class="min-w-0">
              <div class="text-xs text-muted mb-1">{{ t('currentVersion') }}</div>
              <div class="flex items-center gap-2 min-w-0">
                <span class="w-2 h-2 rounded-full bg-success shrink-0"></span>
                <Transition name="fade" mode="out-in">
                  <span :key="currentBranch?.name" class="text-base text-ink font-semibold truncate">
                    {{ currentBranch?.name ?? t('unknown') }}
                  </span>
                </Transition>
              </div>
              <div class="text-xs text-muted truncate mt-1">{{ selectedProject?.path }}</div>
            </div>
            <div class="flex flex-col items-end gap-1 shrink-0">
              <!-- Nudged right so it lines up with the panel edge. -->
              <div class="flex translate-x-1.5 gap-1.5"><CloudDownloadButton /><CloudSyncButton /></div>
              <span class="text-[11px] text-muted">{{ branches.length }} {{ t('versionsUnit') }}</span>
            </div>
          </div>
        </div>

        <VersionList v-model:query="branchQuery" />

        <div class="flex-1 min-h-0 flex flex-col overflow-hidden">
          <CommitLogPanel />
          <GitStatusPanel />
        </div>
      </div>
      <div v-else-if="projects.length > 0 && !loading" key="empty" class="flex-1 flex items-center justify-center px-4 text-center">
        <span class="font-serif text-[20px] font-semibold text-muted leading-tight">{{ t('selectProjectFirst') }}</span>
      </div>
    </Transition>
  </aside>
</template>
