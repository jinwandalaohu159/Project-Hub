<script setup lang="ts">
import { ref } from "vue";
import { Download, Loader2, Sparkles, X } from "lucide-vue-next";
import ModalShell from "./ModalShell.vue";
import { useUpdater } from "../composables/useUpdater";
import { useToast } from "../composables/useToast";
import { useI18n } from "../composables/useI18n";
import { errorText } from "../utils/errors";

const { info, currentVersion, showBanner, installing, progress, install, dismiss } = useUpdater();
const toast = useToast();
const { t } = useI18n();
const showNotes = ref(false);

function formatSize(bytes: number) {
  return bytes > 0 ? `${(bytes / 1024 / 1024).toFixed(1)} MB` : "";
}

async function update() {
  showNotes.value = false;
  try {
    const result = await install();
    if (result === "opened") toast.info(t("updateOpenedInstaller"));
  } catch (error) {
    toast.error(errorText(error, t), { detail: t("updateFailed") });
  }
}
</script>

<template>
  <Transition name="banner">
    <div v-if="showBanner && info" class="update-banner">
      <Sparkles :size="15" class="shrink-0 text-primary" />
      <span class="min-w-0 truncate">
        {{ t('updateAvailable', { version: info.latest }) }}
        <span class="text-muted">{{ t('updateCurrent', { version: currentVersion || info.current }) }}</span>
      </span>
      <button v-if="info.notes && !installing" class="banner-link" @click="showNotes = true">{{ t('updateWhatsNew') }}</button>
      <div class="flex-1" />
      <template v-if="installing">
        <div class="banner-progress"><span :style="{ transform: `scaleX(${progress})` }" /></div>
        <span class="text-xs text-muted tabular-nums w-10 text-right">{{ Math.round(progress * 100) }}%</span>
        <Loader2 :size="14" class="spin text-primary" />
      </template>
      <template v-else>
        <button class="btn btn-ghost btn-sm" @click="dismiss">{{ t('updateLater') }}</button>
        <button class="btn btn-primary btn-sm" @click="update">
          <Download :size="13" />{{ t('updateNow') }}
        </button>
        <button class="banner-close" :title="t('close')" @click="dismiss"><X :size="14" /></button>
      </template>
    </div>
  </Transition>

  <ModalShell
    :open="showNotes"
    :title="t('updateNotesTitle', { version: info?.latest ?? '' })"
    :subtitle="info?.published_at ? new Date(info.published_at).toLocaleString() : undefined"
    width="520px"
    @close="showNotes = false"
  >
    <p class="whitespace-pre-line break-words text-sm leading-6 text-ink">{{ info?.notes }}</p>
    <template #footer>
      <span class="mr-auto self-center text-[11px] text-text-muted">{{ formatSize(info?.asset_size ?? 0) }}</span>
      <button class="btn btn-ghost" @click="showNotes = false">{{ t('close') }}</button>
      <button class="btn btn-primary" @click="update"><Download :size="14" />{{ t('updateNow') }}</button>
    </template>
  </ModalShell>
</template>

<style scoped>
.update-banner {
  display: flex;
  flex: none;
  align-items: center;
  gap: 10px;
  height: 40px;
  overflow: hidden;
  border-bottom: 1px solid color-mix(in srgb, var(--color-primary) 30%, var(--color-border));
  background: color-mix(in srgb, var(--color-primary) 9%, var(--color-surface));
  padding: 0 10px 0 14px;
  color: var(--color-ink);
  font-size: 13px;
}

.banner-link {
  flex: none;
  color: var(--color-primary);
  font-size: 12.5px;
}

.banner-link:hover {
  color: var(--color-primary-hover);
}

.banner-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 6px;
  color: var(--color-muted);
}

.banner-close:hover {
  background: var(--color-surface-raised);
  color: var(--color-ink);
}

.banner-progress {
  width: 160px;
  height: 4px;
  overflow: hidden;
  border-radius: 999px;
  background: var(--color-surface-raised);
}

.banner-progress span {
  display: block;
  height: 100%;
  background: var(--color-primary);
  transform-origin: left;
  transition: transform var(--dur) var(--ease-out);
}

.banner-enter-active,
.banner-leave-active {
  transition: height var(--dur-slow) var(--ease-out), opacity var(--dur) var(--ease-out);
}

.banner-enter-from,
.banner-leave-to {
  height: 0;
  opacity: 0;
}
</style>
