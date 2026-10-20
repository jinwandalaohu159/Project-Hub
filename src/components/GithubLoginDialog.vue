<script setup lang="ts">
import { ref, watch } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { ExternalLink, Github, KeyRound, Loader2 } from "lucide-vue-next";
import ModalShell from "./ModalShell.vue";
import { useAccount } from "../composables/useAccount";
import { useI18n } from "../composables/useI18n";
import { useToast } from "../composables/useToast";
import { errorText, isErrorCode } from "../utils/errors";
import type { GithubAccount } from "../types";

const TOKEN_URL = "https://github.com/settings/tokens/new?scopes=repo&description=Project%20Hub";

const { loginDialogOpen, loginWithBrowser, loginWithToken, closeLoginDialog } = useAccount();
const { t } = useI18n();
const toast = useToast();

const mode = ref<"browser" | "token">("browser");
const busy = ref(false);
const token = ref("");
const error = ref("");

watch(loginDialogOpen, (open) => {
  if (open) {
    mode.value = "browser";
    token.value = "";
    error.value = "";
  }
});

async function signIn(run: () => Promise<GithubAccount | null>) {
  busy.value = true;
  error.value = "";
  try {
    const account = await run();
    if (account) toast.success(t("githubSignedIn", { login: account.login }));
    closeLoginDialog(account);
  } catch (e) {
    if (isErrorCode(e, "GCM_NOT_AVAILABLE")) mode.value = "token";
    error.value = errorText(e, t);
  } finally {
    busy.value = false;
  }
}

function browserLogin() {
  signIn(loginWithBrowser);
}

function tokenLogin() {
  if (!token.value.trim()) return;
  signIn(() => loginWithToken(token.value.trim()));
}

function cancel() {
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
      <div v-if="mode === 'browser'" key="browser" class="space-y-3">
        <button class="btn btn-primary w-full h-10" :disabled="busy" data-autofocus @click="browserLogin">
          <Loader2 v-if="busy" :size="16" class="spin" />
          <Github v-else :size="16" />
          {{ busy ? t('githubWaitingBrowser') : t('githubLoginBrowser') }}
        </button>
        <p class="text-xs leading-5 text-muted">{{ t('githubLoginBrowserHint') }}</p>
        <button class="text-xs text-primary hover:text-primary-hover inline-flex items-center gap-1" :disabled="busy" @click="mode = 'token'">
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
