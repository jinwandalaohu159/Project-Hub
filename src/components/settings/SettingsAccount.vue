<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { Github, Loader2, LogOut } from "lucide-vue-next";
import { useAccount } from "../../composables/useAccount";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";
import { useI18n } from "../../composables/useI18n";
import { errorText } from "../../utils/errors";

const { settings, account, ensureLoaded, refresh, requestLogin, logout, updateAuthor } = useAccount();
const { confirm } = useConfirm();
const toast = useToast();
const { t } = useI18n();

const authorName = ref("");
const authorEmail = ref("");
const savingAuthor = ref(false);
const loggingOut = ref(false);
let authorTimer: ReturnType<typeof setTimeout> | null = null;

const usesNoreply = computed(() =>
  (settings.value?.effective_email ?? "").toLowerCase().endsWith("@users.noreply.github.com")
);

function storedAuthor() {
  return { name: settings.value?.author_name ?? "", email: settings.value?.author_email ?? "" };
}

function isUnchanged() {
  const stored = storedAuthor();
  return authorName.value.trim() === stored.name && authorEmail.value.trim() === stored.email;
}

onMounted(async () => {
  await ensureLoaded();
  await refresh().catch(() => undefined);
});

// Follow outside updates (first load, imports) unless the user is mid-edit.
watch(
  () => [settings.value?.author_name, settings.value?.author_email],
  () => {
    if (authorTimer) return;
    const stored = storedAuthor();
    authorName.value = stored.name;
    authorEmail.value = stored.email;
  },
  { immediate: true }
);

watch([authorName, authorEmail], () => {
  if (authorTimer) clearTimeout(authorTimer);
  authorTimer = null;
  if (isUnchanged()) return;
  authorTimer = setTimeout(saveAuthor, 700);
});

async function saveAuthor() {
  authorTimer = null;
  if (isUnchanged()) return;
  savingAuthor.value = true;
  try {
    await updateAuthor(authorName.value, authorEmail.value);
  } catch (error) {
    toast.error(errorText(error, t));
  } finally {
    savingAuthor.value = false;
  }
}

async function signOut() {
  if (!account.value) return;
  const ok = await confirm({
    title: t("githubLogout"),
    message: t("githubLogoutConfirm", { login: account.value.login }),
    confirmText: t("githubLogout"),
    danger: true,
  });
  if (!ok) return;
  loggingOut.value = true;
  try {
    await logout();
    toast.success(t("githubSignedOut"));
  } catch (error) {
    toast.error(errorText(error, t));
  } finally {
    loggingOut.value = false;
  }
}
</script>

<template>
  <div class="setting-group">
    <label class="setting-label">{{ t('githubAccount') }}</label>
    <div class="setting-card">
      <Transition name="fade" mode="out-in">
        <div v-if="account" key="in" class="flex items-center gap-3">
          <img
            v-if="account.avatar_url"
            :src="account.avatar_url"
            class="h-9 w-9 rounded-full border border-border"
            alt=""
          />
          <Github v-else :size="28" class="text-muted" />
          <div class="min-w-0 flex-1">
            <div class="truncate text-sm font-semibold text-ink">{{ account.name || account.login }}</div>
            <div class="truncate text-xs text-muted">@{{ account.login }}</div>
          </div>
          <button class="btn btn-ghost btn-sm" :disabled="loggingOut" @click="signOut">
            <Loader2 v-if="loggingOut" :size="13" class="spin" />
            <LogOut v-else :size="13" />
            {{ t('githubLogout') }}
          </button>
        </div>
        <div v-else key="out" class="flex items-center justify-between gap-3">
          <div class="setting-description mt-0">{{ t('githubNotSignedIn') }}</div>
          <button class="btn btn-primary btn-sm shrink-0" @click="requestLogin">
            <Github :size="14" />{{ t('githubSignIn') }}
          </button>
        </div>
      </Transition>
    </div>
  </div>

  <div class="setting-group">
    <label class="setting-label">{{ t('authorSection') }}</label>
    <div class="setting-card space-y-3">
      <div class="setting-description mt-0">{{ t('authorDescription') }}</div>
      <label class="number-field">
        <span>{{ t('authorName') }}</span>
        <input
          v-model="authorName"
          type="text"
          :placeholder="account ? (account.name || account.login) : 'Project Hub'"
        />
      </label>
      <label class="number-field">
        <span>{{ t('authorEmail') }}</span>
        <input
          v-model="authorEmail"
          type="email"
          :placeholder="account ? t('authorEmailNoreplyPlaceholder') : 'project-hub@local'"
        />
      </label>
      <div class="flex items-center justify-between gap-2 text-[11px] text-muted">
        <span class="truncate">
          {{ t('authorPreview') }}
          <span class="text-ink">{{ settings?.effective_name || t('authorFromGit') }}</span>
          <span v-if="usesNoreply" class="noreply-chip" :title="`${settings?.effective_email}\n${t('authorNoreplyHint')}`">{{ t('authorNoreply') }}</span>
          <span v-else-if="settings?.effective_email"> &lt;{{ settings.effective_email }}&gt;</span>
        </span>
        <Transition name="fade">
          <Loader2 v-if="savingAuthor" :size="12" class="spin shrink-0" />
        </Transition>
      </div>
    </div>
  </div>
</template>

<style scoped>
.noreply-chip {
  margin-left: 4px;
  border-radius: 999px;
  background: var(--color-surface-raised);
  padding: 1px 7px;
  color: var(--color-muted);
  cursor: help;
}
</style>
