<script setup lang="ts">
import { ref, watch } from "vue";
import { Eye, EyeOff, Trash2 } from "lucide-vue-next";
import ModalShell from "../ModalShell.vue";
import Tooltip from "../Tooltip.vue";
import OpenerIcon from "./OpenerIcon.vue";
import CustomOpenerForm from "./CustomOpenerForm.vue";
import { useI18n } from "../../composables/useI18n";
import { useOpeners } from "../../composables/useOpeners";

/** Managing the programs folders open with: what the menu shows, and the user's own. */
const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: [] }>();

const { t } = useI18n();
const { manageSections, load, setHidden, removeCustom } = useOpeners();
const form = ref<InstanceType<typeof CustomOpenerForm> | null>(null);

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    form.value?.clear();
    load().catch(() => undefined);
  },
);
</script>

<template>
  <ModalShell :open="open" :title="t('openWith')" :subtitle="t('openersSubtitle')" width="540px" @close="emit('close')">
    <div class="space-y-4">
      <section v-for="section in manageSections" :key="section.kind">
        <h4 class="section-title">{{ t(section.title) }}</h4>
        <div class="opener-list">
          <div v-for="opener in section.items" :key="opener.id" class="opener-row" :class="{ 'is-hidden': opener.hidden }">
            <span class="opener-icon"><OpenerIcon :kind="opener.kind" :size="15" /></span>
            <div class="opener-text">
              <div class="truncate text-[13px] text-ink">{{ opener.name }}</div>
              <div class="truncate text-[11px] text-muted" :title="opener.program">{{ opener.program }}</div>
            </div>
            <Tooltip :text="opener.hidden ? t('openerShow') : t('openerHide')">
              <button class="opener-action" @click="setHidden(opener.id, !opener.hidden)">
                <EyeOff v-if="opener.hidden" :size="14" />
                <Eye v-else :size="14" />
              </button>
            </Tooltip>
            <Tooltip v-if="opener.custom" :text="t('openerRemove', { name: opener.name })">
              <button class="opener-action opener-danger" @click="removeCustom(opener.id)"><Trash2 :size="14" /></button>
            </Tooltip>
          </div>
        </div>
      </section>

      <section>
        <h4 class="section-title">{{ t('openerAddTitle') }}</h4>
        <CustomOpenerForm ref="form" />
      </section>
    </div>
  </ModalShell>
</template>

<style scoped>
.section-title {
  margin-bottom: 6px;
  color: var(--color-muted);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.opener-list {
  overflow: hidden;
  border: 1px solid var(--color-border-subtle);
  border-radius: 8px;
}

.opener-row {
  display: flex;
  align-items: center;
  gap: 10px;
  background: var(--color-canvas);
  padding: 7px 8px 7px 10px;
}

.opener-row + .opener-row {
  border-top: 1px solid var(--color-border-subtle);
}

.opener-text {
  min-width: 0;
  flex: 1;
}

/* Hidden from the menu: dimmed, but its buttons stay usable. */
.opener-row.is-hidden .opener-icon,
.opener-row.is-hidden .opener-text {
  opacity: 0.5;
}

.opener-icon {
  display: flex;
  flex: none;
  color: var(--color-muted);
}

.opener-action {
  display: flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 6px;
  color: var(--color-muted);
  transition: background-color var(--dur-fast) ease, color var(--dur-fast) ease;
}

.opener-action:hover {
  background: var(--color-surface-raised);
  color: var(--color-ink);
}

.opener-danger:hover {
  background: color-mix(in srgb, var(--color-error) 12%, transparent);
  color: var(--color-error);
}
</style>
