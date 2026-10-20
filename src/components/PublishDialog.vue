<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Globe, Link2, Loader2, Lock, Plus, Search } from "lucide-vue-next";
import ModalShell from "./ModalShell.vue";
import { useAccount } from "../composables/useAccount";
import { useI18n } from "../composables/useI18n";
import { errorText } from "../utils/errors";
import type { RepoInfo } from "../types";

const props = defineProps<{
  open: boolean;
  projectName: string;
  busy: boolean;
}>();

const emit = defineEmits<{
  create: [input: { name: string; description: string; private: boolean }];
  link: [url: string];
  close: [];
}>();

const { account } = useAccount();
const { t } = useI18n();

const mode = ref<"create" | "link">("create");
const name = ref("");
const description = ref("");
const isPrivate = ref(true);
const linkUrl = ref("");
const repos = ref<RepoInfo[]>([]);
const reposLoading = ref(false);
const reposError = ref("");
const repoQuery = ref("");

watch(
  () => props.open,
  async (open) => {
    if (!open) return;
    mode.value = "create";
    description.value = "";
    isPrivate.value = true;
    linkUrl.value = "";
    repoQuery.value = "";
    name.value = await invoke<string>("suggest_repo_name", { name: props.projectName }).catch(() => props.projectName);
  }
);

// Another account may sign in later; its repositories differ.
watch(() => account.value?.login, () => {
  repos.value = [];
});

watch(mode, async (value) => {
  if (value !== "link" || repos.value.length > 0 || reposLoading.value) return;
  reposLoading.value = true;
  reposError.value = "";
  try {
    repos.value = await invoke<RepoInfo[]>("github_list_repos");
  } catch (error) {
    reposError.value = errorText(error, t);
  } finally {
    reposLoading.value = false;
  }
});

const filteredRepos = computed(() => {
  const query = repoQuery.value.trim().toLowerCase();
  return repos.value.filter((repo) => !query || repo.full_name.toLowerCase().includes(query)).slice(0, 50);
});

const canSubmit = computed(() =>
  mode.value === "create" ? name.value.trim().length > 0 : linkUrl.value.trim().length > 0
);

function submit() {
  if (!canSubmit.value || props.busy) return;
  if (mode.value === "create") {
    emit("create", { name: name.value.trim(), description: description.value.trim(), private: isPrivate.value });
  } else {
    emit("link", linkUrl.value.trim());
  }
}
</script>

<template>
  <ModalShell
    :open="open"
    :busy="busy"
    :title="t('publishTitle')"
    :subtitle="account ? t('publishSubtitle', { login: account.login }) : undefined"
    width="500px"
    @close="emit('close')"
  >
    <div class="segmented mb-4">
      <button :class="{ active: mode === 'create' }" :disabled="busy" @click="mode = 'create'">
        <Plus :size="14" />{{ t('publishCreateTab') }}
      </button>
      <button :class="{ active: mode === 'link' }" :disabled="busy" @click="mode = 'link'">
        <Link2 :size="14" />{{ t('publishLinkTab') }}
      </button>
    </div>

    <Transition name="fade" mode="out-in">
      <form v-if="mode === 'create'" key="create" class="space-y-3" @submit.prevent="submit">
        <div>
          <label class="field-label">{{ t('publishRepoName') }}</label>
          <div class="flex items-center gap-2">
            <span class="text-sm text-muted shrink-0">{{ account?.login }}/</span>
            <input v-model="name" class="field" :disabled="busy" data-autofocus />
          </div>
        </div>
        <div>
          <label class="field-label">{{ t('publishRepoDescription') }}</label>
          <input v-model="description" class="field" :disabled="busy" :placeholder="t('optional')" />
        </div>
        <div class="grid grid-cols-2 gap-2">
          <button type="button" class="visibility-option" :class="{ active: isPrivate }" :disabled="busy" @click="isPrivate = true">
            <Lock :size="15" />
            <span>
              <span class="block text-sm text-ink">{{ t('publishPrivate') }}</span>
              <span class="block text-[11px] text-muted">{{ t('publishPrivateHint') }}</span>
            </span>
          </button>
          <button type="button" class="visibility-option" :class="{ active: !isPrivate }" :disabled="busy" @click="isPrivate = false">
            <Globe :size="15" />
            <span>
              <span class="block text-sm text-ink">{{ t('publishPublic') }}</span>
              <span class="block text-[11px] text-muted">{{ t('publishPublicHint') }}</span>
            </span>
          </button>
        </div>
      </form>

      <div v-else key="link" class="space-y-3">
        <div>
          <label class="field-label">{{ t('publishLinkUrl') }}</label>
          <input
            v-model="linkUrl"
            class="field"
            placeholder="owner/repo  ·  https://github.com/owner/repo"
            :disabled="busy"
            data-autofocus
            @keydown.enter.prevent="submit"
          />
        </div>
        <div class="rounded-lg border border-border-subtle bg-canvas">
          <div class="relative border-b border-border-subtle">
            <Search :size="13" class="absolute left-3 top-1/2 -translate-y-1/2 text-muted" />
            <input v-model="repoQuery" class="w-full bg-transparent py-2 pl-8 pr-3 text-xs outline-none" :placeholder="t('publishSearchRepos')" />
          </div>
          <div class="max-h-44 overflow-y-auto py-1">
            <div v-if="reposLoading" class="flex items-center justify-center gap-2 py-4 text-xs text-muted">
              <Loader2 :size="13" class="spin" />{{ t('loading') }}
            </div>
            <div v-else-if="reposError" class="px-3 py-3 text-xs text-error">{{ reposError }}</div>
            <div v-else-if="filteredRepos.length === 0" class="px-3 py-3 text-xs text-muted">{{ t('publishNoRepos') }}</div>
            <button
              v-for="repo in filteredRepos"
              v-else
              :key="repo.full_name"
              class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs hover:bg-surface-raised"
              :class="linkUrl === repo.clone_url ? 'text-primary' : 'text-ink'"
              @click="linkUrl = repo.clone_url"
            >
              <Lock v-if="repo.private" :size="12" class="text-muted shrink-0" />
              <Globe v-else :size="12" class="text-muted shrink-0" />
              <span class="truncate">{{ repo.full_name }}</span>
            </button>
          </div>
        </div>
        <p class="text-[11px] leading-5 text-muted">{{ t('publishLinkHint') }}</p>
      </div>
    </Transition>

    <template #footer>
      <button class="btn btn-ghost" :disabled="busy" @click="emit('close')">{{ t('cancel') }}</button>
      <button class="btn btn-primary" :disabled="busy || !canSubmit" @click="submit">
        <Loader2 v-if="busy" :size="14" class="spin" />
        {{ busy ? t('uploading') : mode === 'create' ? t('publishCreateAndUpload') : t('publishLinkAndUpload') }}
      </button>
    </template>
  </ModalShell>
</template>

<style scoped>
.segmented {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 4px;
  border-radius: 10px;
  background: var(--color-canvas);
  padding: 3px;
}

.segmented button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  height: 30px;
  border-radius: 8px;
  color: var(--color-muted);
  font-size: 12.5px;
}

.segmented button.active {
  background: var(--color-surface-raised);
  color: var(--color-ink);
  box-shadow: 0 1px 4px rgb(0 0 0 / 0.18);
}

.visibility-option {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: var(--color-canvas);
  padding: 10px;
  text-align: left;
  color: var(--color-muted);
}

.visibility-option.active {
  border-color: var(--color-primary);
  color: var(--color-primary);
  background: color-mix(in srgb, var(--color-primary) 8%, var(--color-canvas));
}
</style>
