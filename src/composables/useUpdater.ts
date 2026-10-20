import { computed, ref, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { usePersistentRef } from "./usePersistentRef";
import { isTauri } from "../utils/env";

export interface UpdateInfo {
  current: string;
  latest: string;
  available: boolean;
  notes: string;
  published_at: string | null;
  release_url: string;
  asset_name: string | null;
  asset_url: string | null;
  asset_size: number;
}

const AUTO_CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;

const currentVersion = ref("");
const releasesUrl = ref("");
const info = ref<UpdateInfo | null>(null);
const checking = ref(false);
const installing = ref(false);
const downloaded = ref(0);
const total = ref(0);
const lastChecked = ref<number | null>(null);
let autoTimer: ReturnType<typeof setInterval> | null = null;
let started = false;
let prefs: { autoCheck: Ref<boolean>; dismissedVersion: Ref<string> } | null = null;

export function useUpdater() {
  prefs ??= {
    autoCheck: usePersistentRef("auto-check-updates", true),
    dismissedVersion: usePersistentRef<string>("dismissed-update", ""),
  };
  const { autoCheck, dismissedVersion } = prefs;

  const available = computed(() => !!info.value?.available);
  /** Banner hides for a version the user dismissed until a newer one appears. */
  const showBanner = computed(() => available.value && info.value?.latest !== dismissedVersion.value);
  const progress = computed(() => (total.value > 0 ? Math.min(1, downloaded.value / total.value) : 0));

  async function loadVersion() {
    if (currentVersion.value || !isTauri()) return;
    const result = await invoke<{ version: string; releases_url: string }>("get_app_version");
    currentVersion.value = result.version;
    releasesUrl.value = result.releases_url;
  }

  async function check() {
    if (checking.value || !isTauri()) return info.value;
    checking.value = true;
    try {
      info.value = await invoke<UpdateInfo>("check_for_update");
      lastChecked.value = Date.now();
      return info.value;
    } finally {
      checking.value = false;
    }
  }

  async function install() {
    const update = info.value;
    if (!update?.asset_url || !update.asset_name || installing.value) return null;
    installing.value = true;
    downloaded.value = 0;
    total.value = update.asset_size;
    const unlisten = await listen<{ downloaded: number; total: number }>("update-progress", (event) => {
      downloaded.value = event.payload.downloaded;
      total.value = event.payload.total || update.asset_size;
    });
    try {
      // On success the app quits and the installer reopens the new version.
      return await invoke<string>("install_update", {
        url: update.asset_url,
        name: update.asset_name,
        size: update.asset_size,
      });
    } finally {
      unlisten();
      installing.value = false;
    }
  }

  function dismiss() {
    if (info.value) dismissedVersion.value = info.value.latest;
  }

  /** Quiet background checks: shortly after start, then every few hours. */
  function startAutoCheck() {
    if (started) return;
    started = true;
    loadVersion().catch(() => undefined);
    const quietCheck = () => {
      if (autoCheck.value) check().catch(() => undefined);
    };
    setTimeout(quietCheck, 5000);
    autoTimer = setInterval(quietCheck, AUTO_CHECK_INTERVAL_MS);
  }

  function stopAutoCheck() {
    if (autoTimer) clearInterval(autoTimer);
    autoTimer = null;
    started = false;
  }

  return {
    currentVersion,
    releasesUrl,
    info,
    checking,
    installing,
    downloaded,
    total,
    progress,
    lastChecked,
    autoCheck,
    available,
    showBanner,
    loadVersion,
    check,
    install,
    dismiss,
    startAutoCheck,
    stopAutoCheck,
  };
}
