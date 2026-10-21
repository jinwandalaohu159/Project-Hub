<script setup lang="ts">
import { computed } from "vue";
import { CloudDownload, Loader2 } from "lucide-vue-next";
import CloudActionButton from "./CloudActionButton.vue";
import ModalShell from "./ModalShell.vue";
import { useCloudSync } from "../composables/useCloudSync";
import { useI18n } from "../composables/useI18n";

/** Brings the cloud's newer save records of the current version to this computer. */
const { sync, downloading, divergedOpen, resolving, busy, start, download, useCloudVersion, useLocalVersion } =
  useCloudSync();
const { t } = useI18n();
start();

const incoming = computed(() => (sync.value?.has_upstream ? sync.value.behind : 0));

const tooltip = computed(() => {
  if (!sync.value?.remote_url) return t("downloadNeedsCloud");
  if (!sync.value.has_upstream) return t("downloadNotInCloud");
  if (incoming.value > 0) return t("downloadPending", { count: incoming.value });
  return t("downloadUpToDate");
});
</script>

<template>
  <CloudActionButton
    :icon="CloudDownload"
    :label="downloading ? t('downloading') : t('downloadFromCloud')"
    :tooltip="tooltip"
    :loading="downloading"
    :disabled="busy || !sync?.remote_url || !sync.has_upstream"
    :badge="incoming > 0 ? `↓${incoming}` : null"
    @click="download"
  />

  <ModalShell
    :open="divergedOpen"
    :busy="resolving !== null"
    :title="t('divergedTitle')"
    width="480px"
    @close="divergedOpen = false"
  >
    <p class="text-sm leading-6 text-ink">
      {{ t('divergedBody', { mine: sync?.ahead ?? 0, cloud: sync?.behind ?? 0, branch: sync?.branch ?? '' }) }}
    </p>
    <div class="mt-4 space-y-2">
      <button class="diverged-choice" :disabled="resolving !== null" @click="useCloudVersion">
        <span class="flex items-center gap-2 font-medium text-ink">
          <Loader2 v-if="resolving === 'cloud'" :size="14" class="spin" />{{ t('divergedUseCloud') }}
        </span>
        <span class="text-xs leading-5 text-muted">{{ t('divergedUseCloudHint') }}</span>
      </button>
      <button class="diverged-choice" :disabled="resolving !== null" @click="useLocalVersion">
        <span class="flex items-center gap-2 font-medium text-ink">
          <Loader2 v-if="resolving === 'mine'" :size="14" class="spin" />{{ t('divergedUseMine') }}
        </span>
        <span class="text-xs leading-5 text-muted">{{ t('divergedUseMineHint') }}</span>
      </button>
    </div>
    <template #footer>
      <button class="btn btn-ghost" :disabled="resolving !== null" @click="divergedOpen = false">{{ t('cancel') }}</button>
    </template>
  </ModalShell>
</template>

<style scoped>
.diverged-choice {
  display: flex;
  width: 100%;
  flex-direction: column;
  gap: 2px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-canvas);
  padding: 10px 12px;
  text-align: left;
  transition: border-color var(--dur-fast) ease, background-color var(--dur-fast) ease;
}

.diverged-choice:hover:not(:disabled) {
  border-color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 6%, var(--color-canvas));
}

.diverged-choice:disabled {
  opacity: 0.6;
}
</style>
