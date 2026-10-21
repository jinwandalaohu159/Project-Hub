<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { ChevronLeft, ChevronRight } from "lucide-vue-next";
import ModalShell from "../ModalShell.vue";
import { useI18n } from "../../composables/useI18n";
import { isReadOnly, useMcpTools } from "../../composables/useMcpTools";
import { MCP_TOOL_DOCS } from "../../utils/mcpToolDocs";
import McpToolBadge from "./McpToolBadge.vue";
import McpToolDetail from "./McpToolDetail.vue";

/** The tools agents can use: a list, and each tool's details one click away. */
const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: [] }>();

const { t } = useI18n();
const { tools, load } = useMcpTools();
const openedName = ref<string | null>(null);
const opened = computed(() => tools.value.find((tool) => tool.name === openedName.value) ?? null);

watch(
  () => props.open,
  (open) => {
    if (open) load();
    else openedName.value = null;
  },
  { immediate: true },
);

function summaryOf(name: string, fallback: string) {
  const doc = MCP_TOOL_DOCS[name];
  return doc ? t(doc.summary) : fallback;
}
</script>

<template>
  <ModalShell :open="open" :title="t('mcpToolsTitle')" :subtitle="t('mcpToolsSubtitle')" width="520px" @close="emit('close')">
    <Transition name="view" mode="out-in">
      <div v-if="opened" key="detail">
        <button class="back-btn" @click="openedName = null">
          <ChevronLeft :size="14" />{{ t('mcpToolBack') }}
        </button>
        <McpToolDetail :tool="opened" />
      </div>
      <div v-else key="list" class="space-y-2">
        <button
          v-for="tool in tools"
          :key="tool.name"
          class="tool-row"
          :aria-label="t('mcpToolOpen', { name: tool.name })"
          @click="openedName = tool.name"
        >
          <div class="flex items-center justify-between gap-2">
            <span class="tool-name">{{ tool.name }}</span>
            <span class="flex items-center gap-1.5">
              <McpToolBadge :read-only="isReadOnly(tool)" />
              <ChevronRight :size="14" class="tool-chevron" />
            </span>
          </div>
          <p class="mt-1 text-xs leading-5 text-muted">{{ summaryOf(tool.name, tool.description) }}</p>
        </button>
      </div>
    </Transition>
  </ModalShell>
</template>

<style scoped>
.tool-row {
  display: block;
  width: 100%;
  border: 1px solid var(--color-border-subtle);
  border-radius: 10px;
  background: var(--color-canvas);
  padding: 10px 12px;
  text-align: left;
  transition: border-color var(--dur-fast) ease, background-color var(--dur-fast) ease;
}

.tool-row:hover {
  border-color: color-mix(in srgb, var(--color-primary) 40%, var(--color-border));
  background: var(--color-surface-raised);
}

.tool-name {
  color: var(--color-ink);
  font-family: "Cascadia Mono", Consolas, "SF Mono", Menlo, monospace;
  font-size: 12.5px;
}

.tool-chevron {
  color: var(--color-muted);
  transition: transform var(--dur-fast) ease;
}

.tool-row:hover .tool-chevron {
  transform: translateX(2px);
}

.back-btn {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  margin-bottom: 12px;
  color: var(--color-muted);
  font-size: 12px;
}

.back-btn:hover {
  color: var(--color-ink);
}

.view-enter-active,
.view-leave-active {
  transition: opacity var(--dur-fast) ease, transform var(--dur-fast) ease;
}

.view-enter-from {
  opacity: 0;
  transform: translateX(8px);
}

.view-leave-to {
  opacity: 0;
  transform: translateX(-8px);
}
</style>
