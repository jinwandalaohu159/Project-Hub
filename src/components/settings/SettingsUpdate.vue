<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Download, ExternalLink, Loader2, RefreshCw } from "lucide-vue-next";
import { useUpdater } from "../../composables/useUpdater";
import { useToast } from "../../composables/useToast";
import { useI18n } from "../../composables/useI18n";
import { errorText } from "../../utils/errors";
import HelpTip from "./HelpTip.vue";

const {
  currentVersion,
  releasesUrl,
  info,
  checking,
  installing,
  available,
  lastChecked,
  autoUpdate,
  loadVersion,
  loadAutoUpdate,
  setAutoUpdate,
  check,
  installNow,
} = useUpdater();
const toast = useToast();
const { t } = useI18n();

onMounted(() => {
  loadVersion().catch(() => undefined);
  loadAutoUpdate().catch(() => undefined);
  if (checking.value) startSpin();
});

// The icon turns while a check runs and always stops on a full turn, so even an instant check
// shows one clear rotation instead of a flicker. The timer is a fallback for when no animation
// events arrive (e.g. the window is hidden).
const SPIN_FALLBACK_MS = 1500;
const spinning = ref(false);
let clickPending = false;
let fallbackTimer: number | undefined;

function startSpin() {
  window.clearTimeout(fallbackTimer);
  spinning.value = true;
}

function stopSpinSoon() {
  window.clearTimeout(fallbackTimer);
  fallbackTimer = window.setTimeout(onSpinTurn, SPIN_FALLBACK_MS);
}

function onSpinTurn() {
  if (!checking.value && !clickPending) spinning.value = false;
}

watch(checking, (busy) => (busy ? startSpin() : stopSpinSoon()));
onBeforeUnmount(() => window.clearTimeout(fallbackTimer));

async function checkNow() {
  if (spinning.value) return;
  startSpin();
  clickPending = true;
  try {
    const result = await check();
    if (result && !result.available) toast.success(t("updateUpToDate", { version: result.current }));
  } catch (error) {
    toast.error(errorText(error, t), { detail: t("updateCheckFailed") });
  } finally {
    clickPending = false;
    stopSpinSoon();
  }
}

async function toggleAutoUpdate() {
  try {
    await setAutoUpdate(!autoUpdate.value);
  } catch (error) {
    toast.error(errorText(error, t));
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
          {{ installing ? t('updateInProgress') : t('updateNow') }}
        </button>
        <button v-else class="btn btn-secondary btn-sm shrink-0" :disabled="spinning" @click="checkNow">
          <span class="inline-flex" :class="{ spin: spinning }" @animationiteration="onSpinTurn"><RefreshCw :size="13" /></span>
          {{ t('updateCheck') }}
        </button>
      </div>
      <div class="flex items-center justify-between gap-3">
        <div class="setting-copy">
          <div class="setting-name">{{ t('updateAuto') }}<HelpTip :text="t('updateAutoDescription')" /></div>
        </div>
        <button class="switch-btn shrink-0" :class="{ active: autoUpdate }" role="switch" :aria-checked="autoUpdate" @click="toggleAutoUpdate">
          <span class="switch-thumb" />
        </button>
      </div>
      <button v-if="releasesUrl" class="text-xs text-muted hover:text-ink inline-flex items-center gap-1" @click="openUrl(releasesUrl)">
        <ExternalLink :size="12" />{{ t('updateReleasesPage') }}
      </button>
    </div>
  </div>
</template>
