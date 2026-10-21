<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Loader2, Plus, X } from "lucide-vue-next";
import ModalShell from "./ModalShell.vue";
import { useI18n } from "../composables/useI18n";
import { useProjectStore } from "../composables/useProjectStore";
import { useIgnoreRules } from "../composables/useIgnoreRules";
import type { IgnoreRules } from "../types";

/** The selected project's hidden files: its .gitignore rules, the defaults, and what they hide. */
const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: [] }>();

const { t } = useI18n();
const { selectedProject } = useProjectStore();
const { load, update } = useIgnoreRules();

const rules = ref<IgnoreRules | null>(null);
const newRule = ref("");
const busy = ref(false);
const moreHidden = computed(() => (rules.value ? rules.value.hidden_total - rules.value.hidden.length : 0));

async function refresh() {
  const project = selectedProject.value;
  if (!project) return;
  try {
    const loaded = await load(project);
    if (project.id === selectedProject.value?.id) rules.value = loaded;
  } catch {
    rules.value = null;
  }
}

watch(
  () => [props.open, selectedProject.value?.id] as const,
  ([open]) => {
    if (!open) return;
    rules.value = null;
    newRule.value = "";
    refresh();
  },
  { immediate: true },
);

async function change(add: string[], remove: string[]) {
  const project = selectedProject.value;
  if (!project || busy.value) return false;
  busy.value = true;
  try {
    const result = await update(project, add, remove);
    if (result) await refresh();
    return result !== null;
  } finally {
    busy.value = false;
  }
}

async function addRule() {
  const rule = newRule.value.trim();
  if (!rule) return;
  if (await change([rule], [])) newRule.value = "";
}
</script>

<template>
  <ModalShell :open="open" :title="t('ignoreRules')" :subtitle="t('ignoreRulesSubtitle')" width="500px" @close="emit('close')">
    <div v-if="!rules" class="flex justify-center py-8 text-muted">
      <Loader2 :size="18" class="spin" />
    </div>
    <div v-else class="space-y-5">
      <section>
        <h4 class="section-title">{{ t('ignoreProjectRules') }}</h4>
        <p class="section-hint">{{ t('ignoreProjectRulesHint') }}</p>
        <form class="add-row" @submit.prevent="addRule">
          <input v-model="newRule" class="rule-input" :placeholder="t('ignoreAddPlaceholder')" :disabled="busy" spellcheck="false" />
          <button class="btn btn-secondary btn-sm shrink-0" type="submit" :disabled="busy || !newRule.trim()">
            <Loader2 v-if="busy" :size="13" class="spin" /><Plus v-else :size="13" />{{ t('ignoreAdd') }}
          </button>
        </form>
        <p v-if="rules.rules.length === 0" class="mt-2 text-xs text-muted">{{ t('ignoreNoRules') }}</p>
        <TransitionGroup v-else name="list" tag="div" class="rule-list">
          <div v-for="rule in rules.rules" :key="rule" class="rule-row">
            <span class="rule-text">{{ rule }}</span>
            <button
              class="rule-remove"
              :disabled="busy"
              :title="t('ignoreRemoveRule', { rule })"
              :aria-label="t('ignoreRemoveRule', { rule })"
              @click="change([], [rule])"
            >
              <X :size="13" />
            </button>
          </div>
        </TransitionGroup>
      </section>

      <section>
        <h4 class="section-title">{{ t('ignoreDefaults') }}</h4>
        <p class="section-hint">{{ t('ignoreDefaultsHint') }}</p>
        <div class="flex flex-wrap gap-1.5">
          <span v-for="rule in rules.defaults" :key="rule" class="rule-chip">{{ rule }}</span>
        </div>
      </section>

      <section>
        <h4 class="section-title">{{ t('ignoreHiddenNow', { count: rules.hidden_total }) }}</h4>
        <p v-if="rules.hidden.length === 0" class="text-xs text-muted">{{ t('ignoreNothingHidden') }}</p>
        <div v-else class="hidden-list">
          <div v-for="path in rules.hidden" :key="path" class="hidden-path">{{ path }}</div>
          <div v-if="moreHidden > 0" class="hidden-more">{{ t('ignoreHiddenMore', { count: moreHidden }) }}</div>
        </div>
      </section>
    </div>
  </ModalShell>
</template>

<style scoped>
.section-title {
  color: var(--color-muted);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.section-hint {
  margin: 2px 0 8px;
  color: var(--color-muted);
  font-size: 11.5px;
  line-height: 1.5;
}

.add-row {
  display: flex;
  gap: 8px;
}

.rule-input {
  min-width: 0;
  flex: 1;
  height: 30px;
  border: 1px solid var(--color-border);
  border-radius: 7px;
  background: var(--color-canvas);
  padding: 0 10px;
  color: var(--color-ink);
  font-family: "Cascadia Mono", Consolas, "SF Mono", Menlo, monospace;
  font-size: 12px;
  outline: none;
  transition: border-color var(--dur-fast) ease, box-shadow var(--dur-fast) ease;
}

.rule-input:focus {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-primary) 18%, transparent);
}

.rule-list {
  position: relative;
  margin-top: 8px;
  overflow: hidden;
  border: 1px solid var(--color-border-subtle);
  border-radius: 8px;
}

.rule-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  background: var(--color-canvas);
  padding: 5px 6px 5px 10px;
}

.rule-row + .rule-row {
  border-top: 1px solid var(--color-border-subtle);
}

.rule-text,
.rule-chip,
.hidden-path {
  font-family: "Cascadia Mono", Consolas, "SF Mono", Menlo, monospace;
  font-size: 12px;
  word-break: break-all;
}

.rule-text {
  color: var(--color-ink);
}

.rule-remove {
  display: flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 5px;
  color: var(--color-muted);
}

.rule-remove:hover:not(:disabled) {
  background: color-mix(in srgb, var(--color-error) 12%, transparent);
  color: var(--color-error);
}

.rule-chip {
  border-radius: 5px;
  background: var(--color-surface-raised);
  padding: 1px 7px;
  color: var(--color-muted);
  font-size: 11.5px;
}

.hidden-list {
  max-height: 180px;
  overflow-y: auto;
  border: 1px solid var(--color-border-subtle);
  border-radius: 8px;
  background: var(--color-canvas);
  padding: 6px 10px;
}

.hidden-path {
  color: var(--color-muted);
  line-height: 1.7;
}

.hidden-more {
  margin-top: 4px;
  color: var(--color-muted);
  font-size: 11px;
}
</style>
