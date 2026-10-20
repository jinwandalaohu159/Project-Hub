<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Loader2 } from "lucide-vue-next";
import ModalShell from "./ModalShell.vue";
import { useI18n } from "../composables/useI18n";

const TITLE_LIMIT = 100;

const props = defineProps<{
  open: boolean;
  busy: boolean;
  title: string;
  description: string;
}>();

const emit = defineEmits<{
  save: [message: { title: string; description: string }];
  close: [];
}>();

const { t } = useI18n();
const titleInput = ref("");
const descriptionInput = ref("");

// Starts from the record's current text each time it opens.
watch(
  () => props.open,
  (open, wasOpen) => {
    if (open && !wasOpen) {
      titleInput.value = props.title;
      descriptionInput.value = props.description;
    }
  }
);

const remaining = computed(() => TITLE_LIMIT - titleInput.value.length);
const unchanged = computed(
  () => titleInput.value.trim() === props.title.trim() && descriptionInput.value.trim() === props.description.trim()
);

function submit() {
  if (props.busy || !titleInput.value.trim()) return;
  if (unchanged.value) {
    emit("close");
    return;
  }
  emit("save", { title: titleInput.value.trim(), description: descriptionInput.value.trim() });
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
    :busy="busy"
    :title="t('editRecordTitle')"
    :subtitle="t('editRecordSubtitle')"
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
          v-model="titleInput"
          class="field"
          :maxlength="TITLE_LIMIT"
          :placeholder="t('saveTitlePlaceholder')"
          :disabled="busy"
          data-autofocus
          @keydown.enter="onTitleEnter"
        />
      </div>
      <div>
        <label class="field-label">{{ t('saveDescriptionLabel') }}</label>
        <textarea
          v-model="descriptionInput"
          class="field resize-y min-h-24 leading-6"
          rows="5"
          :placeholder="t('saveDescriptionPlaceholder')"
          :disabled="busy"
        />
      </div>
    </div>
    <template #footer>
      <span class="mr-auto self-center text-[11px] text-text-muted">{{ t('saveShortcutHint') }}</span>
      <button class="btn btn-ghost" :disabled="busy" @click="emit('close')">{{ t('cancel') }}</button>
      <button class="btn btn-primary min-w-24" :disabled="busy || !titleInput.trim()" @click="submit">
        <Loader2 v-if="busy" :size="14" class="spin" />
        {{ t('saveChanges') }}
      </button>
    </template>
  </ModalShell>
</template>
