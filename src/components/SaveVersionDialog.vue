<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { ChevronRight, Loader2 } from "lucide-vue-next";
import ModalShell from "./ModalShell.vue";
import FileStatusChip from "./FileStatusChip.vue";
import { useI18n } from "../composables/useI18n";
import type { FileDiff } from "../types";
import { changeDisplayPath } from "../utils/path";

const TITLE_LIMIT = 100;
const SLOW_SAVE_MS = 2500;

const props = defineProps<{
  open: boolean;
  saving: boolean;
  changes: FileDiff[];
  total: number;
  branch: string;
}>();

const emit = defineEmits<{
  save: [message: { title: string; description: string }];
  close: [];
}>();

const { t } = useI18n();
const title = ref("");
const description = ref("");
const showFiles = ref(false);

// A fresh dialog each time; an error keeps the text so it can be retried.
watch(
  () => props.open,
  (open, wasOpen) => {
    if (open && !wasOpen) {
      title.value = "";
      description.value = "";
      showFiles.value = false;
    }
  }
);

const remaining = computed(() => TITLE_LIMIT - title.value.length);

// The first save of a big project stores every file once; say so instead of looking stuck.
const slowSave = ref(false);
let slowTimer: ReturnType<typeof setTimeout> | null = null;
watch(
  () => props.saving,
  (saving) => {
    if (slowTimer) clearTimeout(slowTimer);
    slowTimer = null;
    slowSave.value = false;
    if (saving) slowTimer = setTimeout(() => (slowSave.value = true), SLOW_SAVE_MS);
  }
);
onBeforeUnmount(() => {
  if (slowTimer) clearTimeout(slowTimer);
});

function submit() {
  if (props.saving) return;
  emit("save", { title: title.value.trim(), description: description.value.trim() });
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
    event.preventDefault();
    submit();
  }
}

function onTitleEnter(event: KeyboardEvent) {
  if (event.isComposing) return;
  event.preventDefault();
  submit();
}
</script>

<template>
  <ModalShell
    :open="open"
    :busy="saving"
    :title="t('saveVersionTitle')"
    :subtitle="t('saveVersionSubtitle', { branch, count: total })"
    width="520px"
    :close-on-backdrop="false"
    @close="emit('close')"
  >
    <div class="space-y-4" @keydown="onKeydown">
      <div>
        <label class="field-label flex items-center justify-between">
          <span>{{ t('saveTitleLabel') }}</span>
          <span class="text-[11px]" :class="remaining < 10 ? 'text-warning' : 'text-text-muted'">{{ remaining }}</span>
        </label>
        <input
          v-model="title"
          class="field"
          :maxlength="TITLE_LIMIT"
          :placeholder="t('saveTitlePlaceholder')"
          :disabled="saving"
          data-autofocus
          @keydown.enter="onTitleEnter"
        />
      </div>
      <div>
        <label class="field-label">{{ t('saveDescriptionLabel') }}</label>
        <textarea
          v-model="description"
          class="field resize-y min-h-24 leading-6"
          rows="5"
          :placeholder="t('saveDescriptionPlaceholder')"
          :disabled="saving"
        />
      </div>
      <div class="rounded-lg border border-border-subtle bg-canvas">
        <button
          class="flex w-full items-center gap-2 px-3 py-2 text-xs text-muted hover:text-ink"
          @click="showFiles = !showFiles"
        >
          <ChevronRight :size="14" class="transition-transform duration-200" :class="{ 'rotate-90': showFiles }" />
          {{ t('saveFilesToggle', { count: total }) }}
        </button>
        <Transition name="rise">
          <div v-if="showFiles" class="max-h-36 overflow-y-auto border-t border-border-subtle px-3 py-2">
            <div v-for="file in changes" :key="file.path" class="flex items-center gap-2 py-0.5 text-xs text-muted">
              <FileStatusChip :status="file.status" />
              <span class="truncate">{{ changeDisplayPath(file) }}</span>
            </div>
            <div v-if="total > changes.length" class="pt-1 text-[11px] text-text-muted">
              {{ t('changesTruncated', { limit: changes.length, total }) }}
            </div>
          </div>
        </Transition>
      </div>
    </div>
    <template #footer>
      <Transition name="fade" mode="out-in">
        <span v-if="slowSave" key="slow" class="mr-auto self-center text-[11px] text-warning">{{ t('savingLargeHint') }}</span>
        <span v-else key="hint" class="mr-auto self-center text-[11px] text-text-muted">{{ t('saveShortcutHint') }}</span>
      </Transition>
      <button class="btn btn-ghost" :disabled="saving" @click="emit('close')">{{ t('cancel') }}</button>
      <button class="btn btn-primary min-w-24" :disabled="saving" @click="submit">
        <Loader2 v-if="saving" :size="14" class="spin" />
        {{ saving ? t('savingVersion') : t('saveVersionConfirm') }}
      </button>
    </template>
  </ModalShell>
</template>
