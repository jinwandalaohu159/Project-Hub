<script setup lang="ts">
import { Download, Loader2, Sparkles, X } from "lucide-vue-next";
import { useUpdater } from "../composables/useUpdater";
import { useI18n } from "../composables/useI18n";

const { info, currentVersion, showBanner, installing, dialogOpen, installNow, dismiss } = useUpdater();
const { t } = useI18n();
</script>

<template>
  <Transition name="banner">
    <div v-if="showBanner && info" class="update-banner">
      <Sparkles :size="15" class="shrink-0 text-primary" />
      <span class="min-w-0 truncate">
        {{ t('updateAvailable', { version: info.latest }) }}
        <span class="text-muted">{{ t('updateCurrent', { version: currentVersion || info.current }) }}</span>
      </span>
      <button v-if="info.notes && !installing" class="banner-link" @click="dialogOpen = true">{{ t('updateWhatsNew') }}</button>
      <div class="flex-1" />
      <button v-if="installing" class="banner-link inline-flex items-center gap-1.5" @click="dialogOpen = true">
        <Loader2 :size="13" class="spin" />{{ t('updateInProgress') }}
      </button>
      <template v-else>
        <button class="btn btn-ghost btn-sm" @click="dismiss">{{ t('updateLater') }}</button>
        <button class="btn btn-primary btn-sm" @click="installNow">
          <Download :size="13" />{{ t('updateNow') }}
        </button>
        <button class="banner-close" :title="t('close')" @click="dismiss"><X :size="14" /></button>
      </template>
    </div>
  </Transition>
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
