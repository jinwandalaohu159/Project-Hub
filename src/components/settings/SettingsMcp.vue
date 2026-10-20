<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Check, Copy, Eye, ListChecks, Loader2, PenLine } from "lucide-vue-next";
import ModalShell from "../ModalShell.vue";
import type { TranslationKey } from "../../composables/useI18n";
import { useToast } from "../../composables/useToast";
import { useI18n } from "../../composables/useI18n";
import { errorText } from "../../utils/errors";
import type { McpStatus } from "../../types";
import { isTauri } from "../../utils/env";

const toast = useToast();
const { t } = useI18n();

const status = ref<McpStatus | null>(null);
const port = ref(47821);
const busy = ref(false);
const copied = ref<string | null>(null);
const showTools = ref(false);

const TOOLS: Array<{ name: string; description: TranslationKey; writes: boolean }> = [
  { name: "list_projects", description: "mcpToolListProjects", writes: false },
  { name: "get_status", description: "mcpToolGetStatus", writes: false },
  { name: "get_diff", description: "mcpToolGetDiff", writes: false },
  { name: "save_version", description: "mcpToolSaveVersion", writes: true },
  { name: "get_history", description: "mcpToolGetHistory", writes: false },
];

async function load() {
  if (!isTauri()) return;
  status.value = await invoke<McpStatus>("get_mcp_status");
  port.value = status.value.port;
}

onMounted(() => {
  load().catch(() => undefined);
});

async function apply(enabled: boolean) {
  if (busy.value) return;
  busy.value = true;
  try {
    status.value = await invoke<McpStatus>("set_mcp_config", { enabled, port: Math.round(Number(port.value) || 0) });
    port.value = status.value.port;
    if (status.value.error) toast.error(status.value.error);
  } catch (error) {
    toast.error(errorText(error, t));
  } finally {
    busy.value = false;
  }
}

const claudeCommand = computed(() =>
  status.value ? `claude mcp add --transport http project-hub ${status.value.url}` : ""
);

const stdioConfig = computed(() =>
  status.value
    ? JSON.stringify({ mcpServers: { "project-hub": { command: status.value.exe_path, args: ["--mcp"] } } }, null, 2)
    : ""
);

async function copy(id: string, text: string) {
  try {
    await navigator.clipboard.writeText(text);
    copied.value = id;
    setTimeout(() => {
      if (copied.value === id) copied.value = null;
    }, 1600);
  } catch (error) {
    toast.error(errorText(error, t));
  }
}

const statusLabel = computed(() => {
  if (!status.value?.enabled) return t("mcpStopped");
  if (status.value.running) return t("mcpRunning");
  return t("mcpFailed");
});
</script>

<template>
  <div class="setting-group">
    <label class="setting-label">{{ t('mcpSection') }}</label>
    <div class="setting-card space-y-3">
      <div class="flex items-center justify-between gap-3">
        <div class="setting-copy">
          <div class="setting-name flex items-center gap-2">
            {{ t('mcpServer') }}
            <span class="mcp-dot" :class="{ on: status?.running, bad: status?.enabled && !status?.running }" />
            <span class="text-[11px] font-normal text-muted">{{ statusLabel }}</span>
          </div>
          <div class="setting-description">{{ t('mcpDescription') }}</div>
          <div class="setting-description">{{ t('mcpLocalOnly') }}</div>
        </div>
        <button
          class="switch-btn"
          :class="{ active: status?.enabled }"
          role="switch"
          :aria-checked="status?.enabled"
          :disabled="busy || !status"
          @click="apply(!status?.enabled)"
        >
          <span class="switch-thumb" />
        </button>
      </div>

      <div class="flex items-end gap-2">
        <label class="number-field flex-1">
          <span>{{ t('mcpPort') }}</span>
          <input v-model.number="port" type="number" min="1024" max="65535" :disabled="busy" />
        </label>
        <button
          class="btn btn-secondary btn-sm"
          :disabled="busy || !status || port === status.port"
          @click="apply(status?.enabled ?? true)"
        >
          <Loader2 v-if="busy" :size="13" class="spin" />{{ t('apply') }}
        </button>
      </div>

      <template v-if="status">
        <div class="mcp-snippet">
          <div class="mcp-snippet-head">
            <span>{{ t('mcpClaudeCode') }}</span>
            <button class="mcp-copy" @click="copy('claude', claudeCommand)">
              <Check v-if="copied === 'claude'" :size="12" /><Copy v-else :size="12" />
            </button>
          </div>
          <code>{{ claudeCommand }}</code>
        </div>
        <div class="mcp-snippet">
          <div class="mcp-snippet-head">
            <span>{{ t('mcpOtherAgents') }}</span>
            <button class="mcp-copy" @click="copy('stdio', stdioConfig)">
              <Check v-if="copied === 'stdio'" :size="12" /><Copy v-else :size="12" />
            </button>
          </div>
          <code>{{ stdioConfig }}</code>
        </div>
        <button class="btn btn-secondary btn-sm w-full" @click="showTools = true">
          <ListChecks :size="14" />{{ t('mcpToolsButton', { count: TOOLS.length }) }}
        </button>
      </template>
    </div>
  </div>

  <ModalShell :open="showTools" :title="t('mcpToolsTitle')" :subtitle="t('mcpToolsSubtitle')" width="480px" @close="showTools = false">
    <div class="space-y-2">
      <div v-for="tool in TOOLS" :key="tool.name" class="tool-row">
        <div class="flex items-center justify-between gap-2">
          <span class="tool-name">{{ tool.name }}</span>
          <span class="tool-badge" :class="{ writes: tool.writes }">
            <PenLine v-if="tool.writes" :size="11" /><Eye v-else :size="11" />
            {{ tool.writes ? t('mcpToolWrites') : t('mcpToolReads') }}
          </span>
        </div>
        <p class="mt-1 text-xs leading-5 text-muted">{{ t(tool.description) }}</p>
      </div>
    </div>
  </ModalShell>
</template>

<style scoped>
.mcp-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--color-text-muted);
  transition: background-color var(--dur) var(--ease-out), box-shadow var(--dur) var(--ease-out);
}

.mcp-dot.on {
  background: var(--color-success);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--color-success) 22%, transparent);
}

.mcp-dot.bad {
  background: var(--color-error);
}

.mcp-snippet {
  overflow: hidden;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-surface);
}

.mcp-snippet-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 6px 4px 10px;
  border-bottom: 1px solid var(--color-border-subtle);
  color: var(--color-muted);
  font-size: 11px;
}

.mcp-copy {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 5px;
  color: var(--color-muted);
}

.mcp-copy:hover {
  background: var(--color-surface-raised);
  color: var(--color-ink);
}

/* Softer than the editor font at small sizes: ClearType-tuned faces, muted ink, wrapped lines. */
.mcp-snippet code {
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

.tool-row {
  border: 1px solid var(--color-border-subtle);
  border-radius: 10px;
  background: var(--color-canvas);
  padding: 10px 12px;
}

.tool-name {
  color: var(--color-ink);
  font-family: "Cascadia Mono", Consolas, "SF Mono", Menlo, monospace;
  font-size: 12.5px;
}

.tool-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  border-radius: 999px;
  background: var(--color-surface-raised);
  padding: 1px 8px;
  color: var(--color-muted);
  font-size: 11px;
}

.tool-badge.writes {
  background: color-mix(in srgb, var(--color-primary) 16%, transparent);
  color: var(--color-primary);
}
</style>
