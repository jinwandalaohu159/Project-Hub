<script setup lang="ts">
import { computed, ref, watch } from "vue";
import ModalShell from "./ModalShell.vue";
import { useConfirm, type ConfirmOptions } from "../composables/useConfirm";
import { useI18n } from "../composables/useI18n";

const { pending, settle } = useConfirm();
const { t } = useI18n();

// Keep the last content while the dialog animates out.
const shown = ref<ConfirmOptions | null>(null);
watch(pending, (value) => {
  if (value) shown.value = value;
});

const open = computed(() => pending.value !== null);
</script>

<template>
  <ModalShell :open="open" :title="shown?.title" width="420px" @close="settle(false)">
    <p v-if="shown?.message" class="text-sm leading-6 text-muted whitespace-pre-line break-words">
      {{ shown.message }}
    </p>
    <p v-if="shown?.note" class="confirm-note">{{ shown.note }}</p>
    <template #footer>
      <button class="btn btn-ghost" @click="settle(false)">{{ shown?.cancelText ?? t('cancel') }}</button>
      <button
        class="btn"
        :class="shown?.danger ? 'btn-danger' : 'btn-primary'"
        data-autofocus
        @click="settle(true)"
      >
        {{ shown?.confirmText ?? t('confirm') }}
      </button>
    </template>
  </ModalShell>
</template>

<style scoped>
.confirm-note {
  margin-top: 12px;
  border: 1px solid color-mix(in srgb, var(--color-info) 28%, transparent);
  border-radius: 8px;
  background: color-mix(in srgb, var(--color-info) 9%, transparent);
  padding: 8px 10px;
  color: var(--color-ink);
  font-size: 12.5px;
  line-height: 1.55;
  white-space: pre-line;
}
</style>
