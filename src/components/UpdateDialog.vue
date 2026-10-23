<script setup lang="ts">
import { computed } from "vue";
import { Download, Loader2 } from "lucide-vue-next";
import ModalShell from "./ModalShell.vue";
import { useUpdater } from "../composables/useUpdater";
import { useI18n } from "../composables/useI18n";
import { formatBytes } from "../utils/format";

/** Confirms an update, then shows it through: download progress, then the restart. */
const { info, currentVersion, dialogOpen, phase, downloaded, total, progress, installNow } = useUpdater();
const { t } = useI18n();

const updating = computed(() => phase.value !== "idle");

const progressLabel = computed(() =>
  phase.value === "restarting"
    ? t("updateRestarting")
    : t("updateDownloadingProgress", { done: formatBytes(downloaded.value), total: total.value > 0 ? formatBytes(total.value) : "" })
);
</script>

<template>
  <ModalShell
    :open="dialogOpen && !!info"
    :title="t('updateDialogTitle', { version: info?.latest ?? '' })"
    :subtitle="info?.published_at ? new Date(info.published_at).toLocaleString() : undefined"
    :busy="updating"
    width="520px"
    @close="dialogOpen = false"
  >
    <p class="text-sm text-muted">{{ t('updateCurrent', { version: currentVersion || info?.current || '' }) }}</p>
    <p v-if="info?.notes" class="update-notes">{{ info.notes }}</p>

    <div v-if="updating" class="mt-4 space-y-1.5">
      <div class="flex items-center justify-between text-xs">
        <span class="inline-flex items-center gap-1.5 text-ink">
          <Loader2 :size="13" class="spin text-primary" />{{ progressLabel }}
        </span>
        <span v-if="phase === 'downloading'" class="tabular-nums text-muted">{{ Math.round(progress * 100) }}%</span>
      </div>
      <div class="update-progress" :class="{ done: phase === 'restarting' }">
        <span :style="{ transform: `scaleX(${phase === 'restarting' ? 1 : progress})` }" />
      </div>
    </div>
    <p v-else class="mt-3 text-xs text-muted">{{ t('updateRestartHint') }}</p>

    <template #footer>
      <span class="mr-auto self-center text-[11px] text-text-muted">{{ info?.asset_size ? formatBytes(info.asset_size) : '' }}</span>
      <button class="btn btn-ghost" :disabled="updating" @click="dialogOpen = false">{{ t('updateLater') }}</button>
      <button class="btn btn-primary" :disabled="updating" @click="installNow">
        <Loader2 v-if="updating" :size="14" class="spin" />
        <Download v-else :size="14" />
        {{ updating ? t('updateInProgress') : t('updateNow') }}
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
  height: 6px;
  overflow: hidden;
  border-radius: 999px;
  background: var(--color-surface-raised);
}

.update-progress span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--color-primary);
  transform-origin: left;
  transition: transform var(--dur) var(--ease-out), background-color var(--dur) var(--ease-out);
}

.update-progress.done span {
  background: var(--color-success);
}
</style>
