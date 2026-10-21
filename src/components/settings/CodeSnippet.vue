<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import { Check, Copy } from "lucide-vue-next";
import { useI18n } from "../../composables/useI18n";
import { useToast } from "../../composables/useToast";
import { errorText } from "../../utils/errors";

/** A labelled block of code (a command, a config, a tool call or its result) with a copy button. */
const props = defineProps<{
  label: string;
  text: string;
  hint?: string;
  /** Long results scroll instead of stretching the page. */
  maxHeight?: string;
}>();

const { t } = useI18n();
const toast = useToast();
const copied = ref(false);
let resetTimer: ReturnType<typeof setTimeout> | undefined;

async function copy() {
  try {
    await navigator.clipboard.writeText(props.text);
    copied.value = true;
    clearTimeout(resetTimer);
    resetTimer = setTimeout(() => (copied.value = false), 1600);
  } catch (error) {
    toast.error(errorText(error, t));
  }
}

onBeforeUnmount(() => clearTimeout(resetTimer));
</script>

<template>
  <div class="snippet">
    <div class="snippet-head">
      <span class="min-w-0 truncate">{{ label }}</span>
      <button class="snippet-copy" :title="t('mcpToolCopy')" @click="copy">
        <Check v-if="copied" :size="12" /><Copy v-else :size="12" />
      </button>
    </div>
    <p v-if="hint" class="snippet-hint">{{ hint }}</p>
    <code :style="maxHeight ? { maxHeight, overflowY: 'auto' } : undefined">{{ text }}</code>
  </div>
</template>

<style scoped>
.snippet {
  overflow: hidden;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-surface);
}

.snippet-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 4px 6px 4px 10px;
  border-bottom: 1px solid var(--color-border-subtle);
  color: var(--color-muted);
  font-size: 11px;
}

.snippet-hint {
  padding: 6px 10px 0;
  color: var(--color-muted);
  font-size: 11px;
  line-height: 1.5;
}

.snippet-copy {
  display: flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 5px;
  color: var(--color-muted);
}

.snippet-copy:hover {
  background: var(--color-surface-raised);
  color: var(--color-ink);
}

/* Softer than the editor font at small sizes: ClearType-tuned faces, muted ink, wrapped lines. */
.snippet code {
  display: block;
  padding: 8px 10px;
  color: color-mix(in srgb, var(--color-ink) 78%, var(--color-muted));
  font-family: "Cascadia Mono", Consolas, "SF Mono", Menlo, "Ubuntu Mono", monospace;
  font-size: 11.5px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
  -webkit-font-smoothing: auto;
}
</style>
