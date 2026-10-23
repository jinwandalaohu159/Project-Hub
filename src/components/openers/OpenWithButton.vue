<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Settings2, SquareArrowOutUpRight } from "lucide-vue-next";
import Tooltip from "../Tooltip.vue";
import ContextMenu from "../ContextMenu.vue";
import OpenerIcon from "./OpenerIcon.vue";
import OpenersDialog from "./OpenersDialog.vue";
import { useI18n } from "../../composables/useI18n";
import { useOpeners } from "../../composables/useOpeners";
import type { OpenerView } from "../../types";

/** A toolbar button whose menu opens the project folder in a terminal, an editor or the file manager. */
const props = defineProps<{ path: string | null; disabled?: boolean }>();

const { t } = useI18n();
const { menuGroups, load, openFolder } = useOpeners();

const buttonRef = ref<HTMLElement | null>(null);
const menuAt = ref<{ x: number; y: number } | null>(null);
const manageOpen = ref(false);

onMounted(() => load().catch(() => undefined));

function toggle() {
  if (menuAt.value) {
    menuAt.value = null;
    return;
  }
  const rect = buttonRef.value?.getBoundingClientRect();
  if (!rect) return;
  menuAt.value = { x: rect.left, y: rect.bottom + 4 };
  // Programs may have been installed or removed since the last look.
  load().catch(() => undefined);
}

function choose(opener: OpenerView) {
  menuAt.value = null;
  if (props.path) openFolder(opener, props.path);
}

function manage() {
  menuAt.value = null;
  manageOpen.value = true;
}
</script>

<template>
  <Tooltip :text="t('openWith')" position="bottom">
    <button
      ref="buttonRef"
      class="w-7 h-7 flex items-center justify-center rounded-md text-muted hover:text-ink hover:bg-surface-raised disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
      :class="{ 'bg-surface-raised text-ink': menuAt }"
      :disabled="disabled || !path"
      aria-haspopup="menu"
      :aria-expanded="!!menuAt"
      @click.stop="toggle"
    >
      <SquareArrowOutUpRight :size="14" />
    </button>
  </Tooltip>

  <ContextMenu :at="menuAt" :anchor="buttonRef" :min-width="210" @close="menuAt = null">
    <template v-for="(group, index) in menuGroups" :key="group.kind">
      <div v-if="index > 0" class="menu-separator" />
      <div class="opener-group">{{ t(group.title) }}</div>
      <button v-for="opener in group.items" :key="opener.id" class="menu-item" @click="choose(opener)">
        <span class="flex text-muted"><OpenerIcon :kind="opener.kind" /></span>
        <span class="min-w-0 flex-1 truncate text-left">{{ opener.name }}</span>
      </button>
    </template>
    <p v-if="menuGroups.length === 0" class="opener-empty">{{ t('openWithNone') }}</p>
    <div class="menu-separator" />
    <button class="menu-item" @click="manage">
      <Settings2 :size="14" class="text-muted" />
      {{ t('openWithManage') }}
    </button>
  </ContextMenu>

  <OpenersDialog :open="manageOpen" @close="manageOpen = false" />
</template>

<style scoped>
.opener-group {
  padding: 4px 14px 2px;
  color: var(--color-muted);
  font-size: 11px;
  font-weight: 600;
}

.opener-empty {
  max-width: 240px;
  padding: 6px 14px;
  color: var(--color-muted);
  font-size: 12px;
  line-height: 1.5;
}
</style>
