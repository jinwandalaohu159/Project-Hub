<script setup lang="ts">
import { computed, nextTick } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useI18n } from "../../composables/useI18n";
import { useTheme } from "../../composables/useTheme";
import { useHelperProgress } from "../../composables/useHelperProgress";
import { errorText } from "../../utils/errors";
import { formatBytes } from "../../utils/format";

/** The update helper's whole window: downloading (real progress), then restarting. */

// Served from public/; bound dynamically so the bundler does not try to import it.
const iconUrl = "/app-icon.png";

const { t } = useI18n();
useTheme();

// The window starts hidden and is shown once it has drawn itself, so it never flashes white.
const { version, progress, fraction } = useHelperProgress(async () => {
  await nextTick();
  const window = getCurrentWindow();
  await window.show();
  await window.setFocus();
});

const label = computed(() => {
  const current = progress.value;
  if (current.phase === "downloading") return t("updaterDownloading");
  if (current.phase === "restarting") return t("updaterRestarting");
  if (current.phase === "done") return t("updaterDone");
  return t("updaterFailed");
});

const detail = computed(() => {
  const current = progress.value;
  if (current.phase === "downloading" && current.total > 0)
    return `${formatBytes(current.downloaded)} / ${formatBytes(current.total)}`;
  if (current.phase === "failed") return errorText(current.error, t);
  return "";
});
</script>

<template>
  <main class="updater" data-tauri-drag-region>
    <div class="updater-head" data-tauri-drag-region>
      <img :src="iconUrl" alt="" class="updater-icon" draggable="false" data-tauri-drag-region />
      <div class="min-w-0" data-tauri-drag-region>
        <h1 class="updater-title" data-tauri-drag-region>{{ t('updaterTitle') }}</h1>
        <p v-if="version" class="updater-version" data-tauri-drag-region>{{ t('updaterTo', { version }) }}</p>
      </div>
    </div>
    <div class="updater-bar" :class="{ failed: progress.phase === 'failed' }" data-tauri-drag-region>
      <div class="updater-fill" :style="{ transform: `scaleX(${fraction})` }" />
    </div>
    <div v-if="progress.phase === 'failed'" class="updater-failure" data-tauri-drag-region>
      <span class="text-error">{{ label }}</span>
      <span class="updater-reason">{{ detail }}</span>
    </div>
    <div v-else class="updater-status" data-tauri-drag-region>
      <span class="truncate">{{ label }}</span>
      <span class="updater-detail">{{ detail || `${Math.round(fraction * 100)}%` }}</span>
    </div>
  </main>
</template>

<style scoped>
.updater {
  display: flex;
  height: 100vh;
  flex-direction: column;
  justify-content: center;
  gap: 14px;
  border: 1px solid var(--color-border);
  background: var(--color-surface);
  padding: 20px 22px;
  user-select: none;
  cursor: default;
}

.updater-head {
  display: flex;
  align-items: center;
  gap: 12px;
}

.updater-icon {
  width: 36px;
  height: 36px;
  border-radius: 8px;
}

.updater-title {
  color: var(--color-ink);
  font-size: 14px;
  font-weight: 600;
}

.updater-version {
  margin-top: 2px;
  color: var(--color-muted);
  font-size: 12px;
}

.updater-bar {
  height: 6px;
  overflow: hidden;
  border-radius: 999px;
  background: var(--color-surface-raised);
}

.updater-fill {
  height: 100%;
  border-radius: inherit;
  background: var(--color-primary);
  transform-origin: left;
  transition: transform 0.25s var(--ease-out);
}

.updater-bar.failed .updater-fill {
  background: var(--color-error);
}

.updater-status {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  color: var(--color-ink);
  font-size: 12px;
}

.updater-failure {
  display: flex;
  flex-direction: column;
  gap: 2px;
  font-size: 12px;
}

.updater-reason {
  display: -webkit-box;
  overflow: hidden;
  color: var(--color-muted);
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.updater-detail {
  flex: none;
  color: var(--color-muted);
  font-variant-numeric: tabular-nums;
}
</style>
