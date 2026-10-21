<script setup lang="ts">
import { Download, Loader2 } from "lucide-vue-next";
import ModalShell from "./ModalShell.vue";
import { useUpdater } from "../composables/useUpdater";
import { useI18n } from "../composables/useI18n";

/** Confirms an update: what changes, how big it is, then downloads, installs and reopens the app. */
const { info, currentVersion, dialogOpen, installing, progress, installNow } = useUpdater();
const { t } = useI18n();

function formatSize(bytes: number) {
  return bytes > 0 ? `${(bytes / 1024 / 1024).toFixed(1)} MB` : "";
}
</script>

<template>
  <ModalShell
    :open="dialogOpen && !!info"
    :title="t('updateDialogTitle', { version: info?.latest ?? '' })"
    :subtitle="info?.published_at ? new Date(info.published_at).toLocaleString() : undefined"
    :busy="installing"
    width="520px"
    @close="dialogOpen = false"
  >
    <p class="text-sm text-muted">{{ t('updateCurrent', { version: currentVersion || info?.current || '' }) }}</p>
    <p v-if="info?.notes" class="update-notes">{{ info.notes }}</p>
    <div v-if="installing" class="mt-4 flex items-center gap-3">
      <div class="update-progress"><span :style="{ transform: `scaleX(${progress})` }" /></div>
      <span class="w-10 text-right text-xs tabular-nums text-muted">{{ Math.round(progress * 100) }}%</span>
    </div>
    <p class="mt-3 text-xs text-muted">{{ t('updateRestartHint') }}</p>

    <template #footer>
      <span class="mr-auto self-center text-[11px] text-text-muted">{{ formatSize(info?.asset_size ?? 0) }}</span>
      <button class="btn btn-ghost" :disabled="installing" @click="dialogOpen = false">{{ t('updateLater') }}</button>
      <button class="btn btn-primary" :disabled="installing" @click="installNow">
        <Loader2 v-if="installing" :size="14" class="spin" />
        <Download v-else :size="14" />
        {{ installing ? t('updateDownloading') : t('updateNow') }}
      </button>
    </template>
  </ModalShell>
</template>

<style scoped>
.update-notes {
  max-height: 320px;
  margin-top: 12px;
  overflow-y: auto;
  border-radius: 8px;
  background: var(--color-surface-raised);
  padding: 10px 12px;
  color: var(--color-ink);
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-line;
  word-break: break-word;
}

.update-progress {
  flex: 1;
  height: 4px;
  overflow: hidden;
  border-radius: 999px;
  background: var(--color-surface-raised);
}

.update-progress span {
  display: block;
  height: 100%;
  background: var(--color-primary);
  transform-origin: left;
  transition: transform var(--dur) var(--ease-out);
}
</style>
