<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { useProjectStore } from "../composables/useProjectStore";
import { useI18n } from "../composables/useI18n";
import { usePersistentRef, clamp } from "../composables/usePersistentRef";
import { useVersionSwitch } from "../composables/useVersionSwitch";
import BranchManager from "./BranchManager.vue";
import PanelResizer from "./PanelResizer.vue";
import SwitchConfirmModal from "./SwitchConfirmModal.vue";
import { Check, Loader2, Search } from "lucide-vue-next";
import { formatRelativeTime } from "../utils/time";

const DEFAULT_BRANCH_LIST_HEIGHT = 96;

// The project's versions: search, management actions and the list to switch between them.

/** The search text; kept by the parent so it survives switching projects. */
const query = defineModel<string>("query", { required: true });

const { branches, switching } = useProjectStore();
const { t } = useI18n();
const { showModal, pendingBranch, unsavedChanges, switchingTarget, requestSwitch, chooseAction, cancel } =
  useVersionSwitch();

const branchListRef = ref<HTMLElement | null>(null);
const branchListHeight = usePersistentRef("branch-list-height", DEFAULT_BRANCH_LIST_HEIGHT);

const currentBranch = computed(() => branches.value.find((branch) => branch.is_current) ?? null);

const filteredBranches = computed(() => {
  const needle = query.value.trim().toLowerCase();
  return branches.value
    .filter((branch) => {
      if (!needle) return true;
      return (
        branch.name.toLowerCase().includes(needle) ||
        (branch.last_commit_message ?? "").toLowerCase().includes(needle)
      );
    })
    .sort((a, b) => Number(b.is_current) - Number(a.is_current));
});

watch(
  () => currentBranch.value?.name,
  async () => {
    await nextTick();
    branchListRef.value?.scrollTo({ top: 0, behavior: "smooth" });
  }
);

function formatTime(iso: string | null): string {
  return formatRelativeTime(iso, t);
}

function onBranchListResize(dy: number) {
  branchListHeight.value = clamp(branchListHeight.value + dy, 44, 420);
}
</script>

<template>
  <div class="shrink-0 pl-5 pr-5 pt-2 pb-1.5 border-b border-border">
    <div class="flex items-center justify-between mb-1.5">
      <h3 class="text-xs font-semibold text-muted uppercase tracking-wider">{{ t('branches') }}</h3>
      <Transition name="fade">
        <span v-if="switching" class="inline-flex items-center gap-1 text-xs text-primary">
          <Loader2 :size="13" class="spin" />
          {{ t('switchingVersion') }}
        </span>
      </Transition>
    </div>
    <div class="relative mb-1.5">
      <Search :size="14" class="absolute left-3 top-1/2 -translate-y-1/2 text-muted pointer-events-none" />
      <input
        v-model="query"
        class="w-full h-8 pl-8 pr-3 text-xs bg-canvas border border-border rounded-md text-ink placeholder-text-muted focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary/30 transition-colors"
        :placeholder="t('searchVersions')"
      />
    </div>
    <BranchManager />
  </div>

  <div ref="branchListRef" class="shrink-0 overflow-y-auto border-b border-border-subtle/70" :style="{ height: `${branchListHeight}px` }">
    <div v-if="branches.length === 0" class="flex flex-col items-center justify-center py-4">
      <span class="font-serif text-[17px] font-semibold text-muted">{{ t('noBranches') }}</span>
    </div>
    <div v-else-if="filteredBranches.length === 0" class="flex flex-col items-center justify-center py-4">
      <span class="font-serif text-[17px] font-semibold text-muted">{{ t('noVersionMatches') }}</span>
    </div>
    <TransitionGroup name="list" tag="div" class="relative">
      <div
        v-for="branch in filteredBranches"
        :key="branch.name"
        class="group pl-5 pr-5 py-2 cursor-pointer border-b border-border-subtle/70 transition-colors"
        :class="[
          branch.is_current ? 'bg-surface-raised' : 'hover:bg-surface-raised',
          switching && switchingTarget !== branch.name ? 'opacity-45 pointer-events-none' : ''
        ]"
        @click="requestSwitch(branch.name)"
      >
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <div class="flex items-center gap-2 min-w-0">
              <span
                v-if="branch.is_current"
                class="w-2 h-2 rounded-full bg-success shrink-0"
              ></span>
              <span
                class="text-sm truncate"
                :class="branch.is_current ? 'text-success font-medium' : 'text-ink'"
              >{{ branch.name }}</span>
            </div>
            <div class="text-xs text-muted truncate mt-1" :class="branch.is_current ? 'ml-4' : ''">
              {{ branch.last_commit_message || t('unknown') }}
            </div>
            <div v-if="branch.last_commit_time" class="text-xs text-muted mt-0.5" :class="branch.is_current ? 'ml-4' : ''">
              {{ formatTime(branch.last_commit_time) }}
            </div>
          </div>
          <div class="shrink-0">
            <span
              v-if="branch.is_current"
              class="inline-flex items-center gap-1 text-xs text-success px-2 py-1 rounded-md bg-success/10"
            >
              <Check :size="13" />
              {{ t('current') }}
            </span>
            <span
              v-else-if="switchingTarget === branch.name"
              class="inline-flex items-center gap-1 text-xs text-primary px-2 py-1 rounded-md bg-primary/10"
            >
              <Loader2 :size="13" class="spin" />
              {{ t('switchingVersion') }}
            </span>
            <span
              v-else
              class="text-xs text-muted opacity-0 group-hover:opacity-100 transition-opacity"
            >
              {{ t('switchToVersion') }}
            </span>
          </div>
        </div>
      </div>
    </TransitionGroup>
  </div>
  <PanelResizer direction="vertical" @resize="onBranchListResize" @reset="branchListHeight = DEFAULT_BRANCH_LIST_HEIGHT" />

  <SwitchConfirmModal
    :open="showModal"
    :changes="unsavedChanges"
    :target="pendingBranch ?? ''"
    @choose="chooseAction"
    @cancel="cancel"
  />
</template>
