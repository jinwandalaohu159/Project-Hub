import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { DeviceCode, GithubAccount, SettingsView } from "../types";
import { isTauri } from "../utils/env";

const settings = ref<SettingsView | null>(null);
const account = ref<GithubAccount | null>(null);
const loginDialogOpen = ref(false);
let loaded: Promise<void> | null = null;
let loginWaiters: Array<(account: GithubAccount | null) => void> = [];

function applySettings(view: SettingsView) {
  settings.value = view;
  account.value = view.github;
}

export function useAccount() {
  async function refresh() {
    if (!isTauri()) return;
    applySettings(await invoke<SettingsView>("get_app_settings"));
  }

  function ensureLoaded() {
    loaded ??= refresh().catch(() => {
      loaded = null;
    });
    return loaded;
  }

  /** Gets the code the user enters on github.com; `finishBrowserLogin` then waits for their approval. */
  function startBrowserLogin() {
    return invoke<DeviceCode>("github_browser_login_start");
  }

  async function finishBrowserLogin() {
    account.value = await invoke<GithubAccount>("github_browser_login_finish");
    await refresh();
    return account.value;
  }

  function cancelBrowserLogin() {
    return invoke("github_browser_login_cancel");
  }

  async function loginWithToken(token: string) {
    account.value = await invoke<GithubAccount>("github_login_token", { token });
    await refresh();
    return account.value;
  }

  async function logout() {
    await invoke("github_logout");
    await refresh();
  }

  async function updateAuthor(name: string, email: string) {
    applySettings(await invoke<SettingsView>("update_author", { name, email }));
  }

  /** Opens the shared sign-in dialog and resolves once it closes (null when cancelled). */
  function requestLogin(): Promise<GithubAccount | null> {
    if (account.value) return Promise.resolve(account.value);
    loginDialogOpen.value = true;
    return new Promise((resolve) => loginWaiters.push(resolve));
  }

  /** The stored token is gone or rejected: forget the account and ask again. */
  function requireFreshLogin(): Promise<GithubAccount | null> {
    account.value = null;
    loginDialogOpen.value = true;
    return new Promise((resolve) => loginWaiters.push(resolve));
  }

  function closeLoginDialog(result: GithubAccount | null) {
    loginDialogOpen.value = false;
    const waiters = loginWaiters;
    loginWaiters = [];
    waiters.forEach((resolve) => resolve(result));
  }

  return {
    settings,
    account,
    loginDialogOpen,
    refresh,
    ensureLoaded,
    startBrowserLogin,
    finishBrowserLogin,
    cancelBrowserLogin,
    loginWithToken,
    logout,
    updateAuthor,
    requestLogin,
    requireFreshLogin,
    closeLoginDialog,
  };
}
