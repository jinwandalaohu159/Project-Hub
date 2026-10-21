<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "../composables/useI18n";
import ModalShell from "./ModalShell.vue";

/** Asks for a file or folder name: `request` resolves with the name, or null when cancelled. */
const emit = defineEmits<{ settled: [] }>();

const { t } = useI18n();
const open = ref(false);
const title = ref("");
const actionLabel = ref("");
const value = ref("");
let resolveRequest: ((value: string | null) => void) | null = null;

function request(dialogTitle: string, defaultValue: string, action: string): Promise<string | null> {
  // A new question replaces one still open.
  resolveRequest?.(null);
  title.value = dialogTitle;
  actionLabel.value = action;
  value.value = defaultValue;
  open.value = true;
  return new Promise((resolve) => {
    resolveRequest = resolve;
  });
}

function settle(result: string | null) {
  const resolve = resolveRequest;
  resolveRequest = null;
  open.value = false;
  resolve?.(result);
  emit("settled");
}

defineExpose({ request });
</script>

<template>
  <ModalShell :open="open" :title="title" width="420px" @close="settle(null)">
    <form id="file-name-form" @submit.prevent="settle(value)">
      <input v-model="value" class="field" spellcheck="false" />
    </form>
    <template #footer>
      <button type="button" class="btn btn-ghost" @click="settle(null)">{{ t('cancel') }}</button>
      <button type="submit" form="file-name-form" class="btn btn-primary" :disabled="!value.trim()">
        {{ actionLabel }}
      </button>
    </template>
  </ModalShell>
</template>
