<script setup lang="ts">
import { Archive, Save, Trash2 } from "lucide-vue-next";
import ModalShell from "./ModalShell.vue";
import FileStatusChip from "./FileStatusChip.vue";
import { useI18n, type TranslationKey } from "../composables/useI18n";
import type { FileDiff } from "../types";
import type { SwitchAction } from "../composables/useProjectStore";

defineProps<{
  open: boolean;
  changes: FileDiff[];
  target: string;
}>();

const emit = defineEmits<{
  choose: [action: SwitchAction];
  cancel: [];
}>();

const { t } = useI18n();

const options: Array<{ action: SwitchAction; icon: typeof Archive; title: TranslationKey; body: TranslationKey; recommended: boolean }> = [
  { action: "draft", icon: Archive, title: "switchKeepDraft", body: "switchKeepDraftBody", recommended: true },
  { action: "save", icon: Save, title: "saveAndSwitch", body: "switchSaveBody", recommended: false },
  { action: "discard", icon: Trash2, title: "discardChanges", body: "switchDiscardBody", recommended: false },
];
</script>

<template>
  <ModalShell
    :open="open"
    :title="t('unsavedChangesDetected')"
    :subtitle="t('switchUnsavedSubtitle', { name: target })"
    width="460px"
    @close="emit('cancel')"
  >
    <div class="mb-3 max-h-28 overflow-y-auto rounded-lg border border-border-subtle bg-canvas p-2">
      <div v-for="file in changes" :key="file.path" class="text-xs text-muted py-0.5 flex items-center gap-2">
        <FileStatusChip :status="file.status" />
        <span class="truncate">{{ file.path }}</span>
      </div>
    </div>
    <div class="flex flex-col gap-2">
      <button
        v-for="option in options"
        :key="option.action"
        class="switch-option"
        :class="{ recommended: option.recommended, danger: option.action === 'discard' }"
        :data-autofocus="option.recommended ? '' : undefined"
        @click="emit('choose', option.action)"
      >
        <component :is="option.icon" :size="16" class="switch-option-icon" />
        <span class="min-w-0 text-left">
          <span class="flex items-center gap-2 text-sm font-medium text-ink">
            {{ t(option.title) }}
            <span v-if="option.recommended" class="recommend-chip">{{ t('recommended') }}</span>
          </span>
          <span class="mt-0.5 block text-xs text-muted leading-5">{{ t(option.body) }}</span>
        </span>
      </button>
    </div>
    <template #footer>
      <button class="btn btn-ghost" @click="emit('cancel')">{{ t('cancel') }}</button>
    </template>
  </ModalShell>
</template>

<style scoped>
.switch-option {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  width: 100%;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: var(--color-canvas);
  padding: 10px 12px;
  outline: none;
}

.switch-option:hover,
.switch-option:focus-visible {
  border-color: color-mix(in srgb, var(--color-primary) 60%, var(--color-border));
  background: var(--color-surface-raised);
}

.switch-option.recommended {
  border-color: color-mix(in srgb, var(--color-primary) 45%, var(--color-border));
}

.switch-option.danger:hover,
.switch-option.danger:focus-visible {
  border-color: color-mix(in srgb, var(--color-error) 60%, var(--color-border));
}

.switch-option-icon {
  flex: none;
  margin-top: 2px;
  color: var(--color-primary);
}

.switch-option.danger .switch-option-icon {
  color: var(--color-error);
}

.recommend-chip {
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-primary) 18%, transparent);
  padding: 0 7px;
  color: var(--color-primary);
  font-size: 11px;
  line-height: 18px;
}
</style>
