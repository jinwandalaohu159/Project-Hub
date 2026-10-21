<script setup lang="ts">
import { ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Check, Copy, ExternalLink, Github, KeyRound, Loader2 } from "lucide-vue-next";
import ModalShell from "./ModalShell.vue";
import { useAccount } from "../composables/useAccount";
import { useI18n } from "../composables/useI18n";
import { useToast } from "../composables/useToast";
import { errorText, isErrorCode } from "../utils/errors";
import type { DeviceCode, GithubAccount } from "../types";

const TOKEN_URL = "https://github.com/settings/tokens/new?scopes=repo&description=Project%20Hub";

const { loginDialogOpen, startBrowserLogin, finishBrowserLogin, cancelBrowserLogin, loginWithToken, closeLoginDialog } =
  useAccount();
const { t } = useI18n();
const toast = useToast();

const mode = ref<"browser" | "token">("browser");
/** Token sign-in in progress; the dialog cannot be closed meanwhile. */
const busy = ref(false);
/** Browser sign-in in progress: asking GitHub for a code, then waiting for the user to approve it. */
const starting = ref(false);
const code = ref<DeviceCode | null>(null);
const copied = ref(false);
const token = ref("");
const error = ref("");

watch(loginDialogOpen, (open) => {
  if (open) {
    mode.value = "browser";
    code.value = null;
    token.value = "";
    error.value = "";
  }
});

function signedIn(account: GithubAccount | null) {
  if (account) toast.success(t("githubSignedIn", { login: account.login }));
  closeLoginDialog(account);
}

async function copyCode() {
  if (!code.value) return;
  try {
    await navigator.clipboard.writeText(code.value.user_code);
    copied.value = true;
  } catch {
    // Some systems only allow copying right after a click; the code stays on screen to copy.
    copied.value = false;
  }
}

/** Shows a code, opens github.com for the user to enter it, and signs in once they approve. */
async function browserLogin() {
  error.value = "";
  starting.value = true;
  try {
    code.value = await startBrowserLogin();
    await copyCode();
    openUrl(code.value.verification_uri);
    signedIn(await finishBrowserLogin());
  } catch (e) {
    if (!isErrorCode(e, "GITHUB_LOGIN_CANCELLED")) error.value = errorText(e, t);
    code.value = null;
  } finally {
    starting.value = false;
  }
}

async function tokenLogin() {
  if (!token.value.trim()) return;
  busy.value = true;
  error.value = "";
  try {
    signedIn(await loginWithToken(token.value.trim()));
  } catch (e) {
    error.value = errorText(e, t);
  } finally {
    busy.value = false;
  }
}

function useToken() {
  if (code.value) cancelBrowserLogin();
  mode.value = "token";
}

function cancel() {
  if (code.value || starting.value) cancelBrowserLogin();
  closeLoginDialog(null);
}
</script>

<template>
  <ModalShell
    :open="loginDialogOpen"
    :busy="busy"
    :title="t('githubLoginTitle')"
    :subtitle="t('githubLoginSubtitle')"
    width="440px"
    @close="cancel"
  >
    <Transition name="fade" mode="out-in">
      <div v-if="mode === 'browser' && code" key="code" class="space-y-3 text-center">
        <p class="text-xs leading-5 text-muted">{{ t('githubDeviceStep') }}</p>
        <button class="device-code" :title="t('githubCodeCopy')" @click="copyCode">{{ code.user_code }}</button>
        <p class="inline-flex items-center gap-1 text-xs" :class="copied ? 'text-success' : 'text-muted'">
          <Check v-if="copied" :size="13" /><Copy v-else :size="13" />
          {{ copied ? t('githubCodeCopied') : t('githubCodeCopy') }}
        </p>
        <div class="flex items-center justify-center gap-2 text-xs text-muted">
          <Loader2 :size="13" class="spin" />{{ t('githubWaitingBrowser') }}
        </div>
        <div class="flex items-center justify-between pt-1">
          <button class="text-xs text-primary hover:text-primary-hover inline-flex items-center gap-1" @click="useToken">
            <KeyRound :size="13" />{{ t('githubUseToken') }}
          </button>
          <button class="btn btn-secondary btn-sm" @click="openUrl(code.verification_uri)">
            <ExternalLink :size="13" />{{ t('githubOpenAgain') }}
          </button>
        </div>
      </div>
      <div v-else-if="mode === 'browser'" key="browser" class="space-y-3">
        <button class="btn btn-primary w-full h-10" :disabled="starting" data-autofocus @click="browserLogin">
          <Loader2 v-if="starting" :size="16" class="spin" />
          <Github v-else :size="16" />
          {{ t('githubLoginBrowser') }}
        </button>
        <p class="text-xs leading-5 text-muted">{{ t('githubLoginBrowserHint') }}</p>
        <button class="text-xs text-primary hover:text-primary-hover inline-flex items-center gap-1" :disabled="starting" @click="useToken">
          <KeyRound :size="13" />{{ t('githubUseToken') }}
        </button>
      </div>
      <form v-else key="token" class="space-y-3" @submit.prevent="tokenLogin">
        <p class="text-xs leading-5 text-muted">{{ t('githubTokenHint') }}</p>
        <button type="button" class="btn btn-secondary btn-sm" @click="openUrl(TOKEN_URL)">
          <ExternalLink :size="13" />{{ t('githubCreateToken') }}
        </button>
        <input
          v-model="token"
          type="password"
          class="field font-mono"
          placeholder="ghp_… / github_pat_…"
          autocomplete="off"
          :disabled="busy"
          data-autofocus
        />
        <div class="flex items-center justify-between">
          <button type="button" class="text-xs text-muted hover:text-ink" :disabled="busy" @click="mode = 'browser'">
            {{ t('githubBackToBrowser') }}
          </button>
          <button type="submit" class="btn btn-primary btn-sm" :disabled="busy || !token.trim()">
            <Loader2 v-if="busy" :size="14" class="spin" />{{ t('githubSignIn') }}
          </button>
        </div>
      </form>
    </Transition>
    <Transition name="rise">
      <p v-if="error" class="mt-3 rounded-lg bg-error/10 px-3 py-2 text-xs leading-5 text-error break-words">{{ error }}</p>
    </Transition>
  </ModalShell>
</template>

<style scoped>
.device-code {
  display: block;
  width: 100%;
  border: 1px dashed var(--color-border);
  border-radius: 10px;
  background: var(--color-surface-raised);
  padding: 14px 0;
  color: var(--color-ink);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 26px;
  font-weight: 600;
  letter-spacing: 0.18em;
  transition: border-color var(--dur-fast, 120ms) ease;
}

.device-code:hover {
  border-color: var(--color-primary);
}
</style>
