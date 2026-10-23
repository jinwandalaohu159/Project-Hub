<script setup lang="ts">
import { ref } from "vue";
import { open as pickFile } from "@tauri-apps/plugin-dialog";
import { Plus } from "lucide-vue-next";
import { useI18n } from "../../composables/useI18n";
import { useOpeners } from "../../composables/useOpeners";

/** Adding a program of the user's own: a name, where it is, and optional arguments. */
const { t } = useI18n();
const { addCustom } = useOpeners();

const name = ref("");
const program = ref("");
const args = ref("");
const adding = ref(false);

function clear() {
  name.value = "";
  program.value = "";
  args.value = "";
}

/** Picks the program; its file name becomes the name unless one was typed. */
async function chooseProgram() {
  const selected = await pickFile({ multiple: false, directory: false });
  if (typeof selected !== "string") return;
  program.value = selected;
  if (!name.value.trim()) {
    name.value = selected.split(/[\\/]/).pop()?.replace(/\.(exe|app|cmd|bat|sh)$/i, "") ?? "";
  }
}

async function add() {
  if (adding.value || !name.value.trim() || !program.value.trim()) return;
  adding.value = true;
  try {
    if (await addCustom(name.value, program.value, args.value)) clear();
  } finally {
    adding.value = false;
  }
}

defineExpose({ clear });
</script>

<template>
  <form class="add-form" @submit.prevent="add">
    <label>
      <span class="field-label">{{ t('openerName') }}</span>
      <input v-model="name" class="field" :placeholder="t('openerNamePlaceholder')" spellcheck="false" />
    </label>
    <label>
      <span class="field-label">{{ t('openerProgram') }}</span>
      <span class="flex gap-2">
        <input v-model="program" class="field min-w-0" :placeholder="t('openerProgramPlaceholder')" spellcheck="false" />
        <button type="button" class="btn btn-secondary shrink-0" @click="chooseProgram">{{ t('openerChoose') }}</button>
      </span>
    </label>
    <label>
      <span class="field-label">{{ t('openerArgs') }}</span>
      <input v-model="args" class="field" placeholder="{path}" spellcheck="false" />
      <span class="field-hint">{{ t('openerArgsHint') }}</span>
    </label>
    <div class="flex justify-end">
      <button type="submit" class="btn btn-primary btn-sm" :disabled="adding || !name.trim() || !program.trim()">
        <Plus :size="13" />{{ t('openerAdd') }}
      </button>
    </div>
  </form>
</template>

<style scoped>
.add-form {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.field-hint {
  display: block;
  margin-top: 4px;
  color: var(--color-muted);
  font-size: 11px;
}
</style>
