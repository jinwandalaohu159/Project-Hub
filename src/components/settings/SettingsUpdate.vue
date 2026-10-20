<script setup lang="ts">
import { computed, onMounted } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Download, ExternalLink, Loader2, RefreshCw } from "lucide-vue-next";
import { useUpdater } from "../../composables/useUpdater";
import { useToast } from "../../composables/useToast";
import { useI18n } from "../../composables/useI18n";
import { errorText } from "../../utils/errors";

const { currentVersion, releasesUrl, info, checking, installing, available, lastChecked, autoCheck, loadVersion, check, install } =
  useUpdater();
const toast = useToast();
const { t } = useI18n();

onMounted(() => {
  loadVersion().catch(() => undefined);
});

async function checkNow() {
  try {
    const result = await check();
    if (result && !result.available) toast.success(t("updateUpToDate", { version: result.current }));
  } catch (error) {
    toast.error(errorText(error, t), { detail: t("updateCheckFailed") });
  }
}

async function installNow() {
  try {
    const result = await install();
    if (result === "opened") toast.info(t("updateOpenedInstaller"));
  } catch (error) {
    toast.error(errorText(error, t), { detail: t("updateFailed") });
  }
}

const status = computed(() => {
  if (checking.value) return t("updateChecking");
  if (available.value && info.value) return t("updateAvailable", { version: info.value.latest });
  if (lastChecked.value) return t("updateLastChecked", { time: new Date(lastChecked.value).toLocaleTimeString() });
  return "";
});
</script>

<template>
  <div class="setting-group">
    <label class="setting-label">{{ t('updateSection') }}</label>
    <div class="setting-card space-y-3">
      <div class="flex items-center justify-between gap-3">
        <div class="setting-copy">
          <div class="setting-name">Project Hub {{ currentVersion ? `v${currentVersion}` : '' }}</div>
          <div class="setting-description" :class="{ 'text-primary': available }">{{ status }}</div>
        </div>
        <button v-if="available" class="btn btn-primary btn-sm shrink-0" :disabled="installing" @click="installNow">
          <Loader2 v-if="installing" :size="13" class="spin" /><Download v-else :size="13" />
          {{ installing ? t('updateDownloading') : t('updateNow') }}
        </button>
        <button v-else class="btn btn-secondary btn-sm shrink-0" :disabled="checking" @click="checkNow">
          <Loader2 v-if="checking" :size="13" class="spin" /><RefreshCw v-else :size="13" />
          {{ t('updateCheck') }}
        </button>
      </div>
      <div class="flex items-center justify-between gap-3">
        <div class="setting-description mt-0">{{ t('updateAutoCheck') }}</div>
        <button class="switch-btn" :class="{ active: autoCheck }" role="switch" :aria-checked="autoCheck" @click="autoCheck = !autoCheck">
          <span class="switch-thumb" />
        </button>
      </div>
      <button v-if="releasesUrl" class="text-xs text-muted hover:text-ink inline-flex items-center gap-1" @click="openUrl(releasesUrl)">
        <ExternalLink :size="12" />{{ t('updateReleasesPage') }}
      </button>
    </div>
  </div>
</template>
