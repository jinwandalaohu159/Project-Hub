import { computed, ref, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { usePersistentRef } from "./usePersistentRef";
import { useToast } from "./useToast";
import { useI18n } from "./useI18n";
import { isTauri } from "../utils/env";
import { errorText } from "../utils/errors";

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

/** Where an update stands once the user starts it; after `restarting` the app quits and the
 *  update helper's window takes over. */
export type UpdatePhase = "idle" | "downloading" | "restarting";

/** Checks are cheap (see update/check.rs), so a new release shows up within minutes. */
const AUTO_CHECK_INTERVAL_MS = 2 * 60 * 1000;
/** Coming back to the window checks right away, unless a check just ran. */
const FOCUS_CHECK_MIN_GAP_MS = 30 * 1000;

const currentVersion = ref("");
const releasesUrl = ref("");
const info = ref<UpdateInfo | null>(null);
const checking = ref(false);
const installing = ref(false);
const phase = ref<UpdatePhase>("idle");
const downloaded = ref(0);
const total = ref(0);
const lastChecked = ref<number | null>(null);
const dialogOpen = ref(false);
/** Installs new versions by itself when the app opens (done by the backend before this loads). */
const autoUpdate = ref(true);
let autoTimer: ReturnType<typeof setInterval> | null = null;
let onFocus: (() => void) | null = null;
let started = false;
let dismissed: Ref<string> | null = null;

export function useUpdater() {
  dismissed ??= usePersistentRef<string>("dismissed-update", "");
  const dismissedVersion = dismissed;

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
    phase.value = "downloading";
    downloaded.value = 0;
    total.value = update.asset_size;
    const unlisten = await listen<{ phase: UpdatePhase; downloaded: number; total: number }>("update-progress", (event) => {
      phase.value = event.payload.phase;
      downloaded.value = event.payload.downloaded;
      total.value = event.payload.total || update.asset_size;
    });
    try {
      // On success the app quits and the installer reopens the new version.
      return await invoke<string>("install_update", {
        version: update.latest,
        url: update.asset_url,
        name: update.asset_name,
        size: update.asset_size,
      });
    } catch (error) {
      phase.value = "idle";
      throw error;
    } finally {
      unlisten();
      installing.value = false;
    }
  }

  /** Installs the available update in the update dialog, which shows the progress; a failure is
   *  reported, and on success the app quits and reopens. */
  async function installNow() {
    dialogOpen.value = true;
    try {
      await install();
    } catch (error) {
      const { t } = useI18n();
      useToast().error(errorText(error, t), { detail: t("updateFailed") });
    }
  }

  async function loadAutoUpdate() {
    if (isTauri()) autoUpdate.value = await invoke<boolean>("get_auto_update");
  }

  async function setAutoUpdate(enabled: boolean) {
    autoUpdate.value = await invoke<boolean>("set_auto_update", { enabled });
  }

  /** Once the app is open again after an update, says how it went. */
  async function reportLastUpdate() {
    const outcome = await invoke<{ version: string; ok: boolean; error: string | null } | null>("take_update_outcome");
    if (!outcome) return;
    const { t } = useI18n();
    if (outcome.ok) useToast().success(t("updateInstalled", { version: outcome.version }));
    else
      useToast().error(errorText(outcome.error, t), { detail: t("updateAutoFailed", { version: outcome.version }) });
  }

  function dismiss() {
    if (info.value) dismissedVersion.value = info.value.latest;
  }

  /** Quiet background checks: shortly after start, every few minutes, and on returning to the window. */
  function startAutoCheck() {
    if (started) return;
    started = true;
    loadVersion().catch(() => undefined);
    loadAutoUpdate().catch(() => undefined);
    reportLastUpdate().catch(() => undefined);
    const quietCheck = () => {
      if (!installing.value) check().catch(() => undefined);
    };
    setTimeout(quietCheck, 5000);
    autoTimer = setInterval(quietCheck, AUTO_CHECK_INTERVAL_MS);
    onFocus = () => {
      if (Date.now() - (lastChecked.value ?? 0) >= FOCUS_CHECK_MIN_GAP_MS) quietCheck();
    };
    window.addEventListener("focus", onFocus);
  }

  function stopAutoCheck() {
    if (autoTimer) clearInterval(autoTimer);
    autoTimer = null;
    if (onFocus) window.removeEventListener("focus", onFocus);
    onFocus = null;
    started = false;
  }

  return {
    currentVersion,
    releasesUrl,
    info,
    checking,
    installing,
    phase,
    downloaded,
    total,
    progress,
    lastChecked,
    autoUpdate,
    dialogOpen,
    available,
    showBanner,
    loadVersion,
    check,
    install,
    installNow,
    dismiss,
    loadAutoUpdate,
    setAutoUpdate,
    startAutoCheck,
    stopAutoCheck,
  };
}
