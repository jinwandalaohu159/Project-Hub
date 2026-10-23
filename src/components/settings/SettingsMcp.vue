<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { ChevronDown, ListChecks, Loader2 } from "lucide-vue-next";
import CodeSnippet from "./CodeSnippet.vue";
import McpToolsDialog from "./McpToolsDialog.vue";
import HelpTip from "./HelpTip.vue";
import { useMcpTools } from "../../composables/useMcpTools";
import { useToast } from "../../composables/useToast";
import { useI18n } from "../../composables/useI18n";
import { errorText } from "../../utils/errors";
import type { McpStatus } from "../../types";
import { isTauri } from "../../utils/env";
import { usePersistentRef } from "../../composables/usePersistentRef";

const toast = useToast();
const { t } = useI18n();

const status = ref<McpStatus | null>(null);
const port = ref(47821);
const busy = ref(false);
const showTools = ref(false);
const { tools, load: loadTools } = useMcpTools();
/** The connection snippets are long and only needed while setting up an agent. */
const showSnippets = usePersistentRef("mcp-snippets-open", false);

async function load() {
  if (!isTauri()) return;
  status.value = await invoke<McpStatus>("get_mcp_status");
  port.value = status.value.port;
}

onMounted(() => {
  load().catch(() => undefined);
  loadTools();
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

// User scope works in every folder; the default (local) scope only in the folder it is run in.
const claudeGlobalCommand = computed(() =>
  status.value ? `claude mcp add --transport http --scope user project-hub ${status.value.url}` : ""
);
const claudeLocalCommand = computed(() =>
  status.value ? `claude mcp add --transport http project-hub ${status.value.url}` : ""
);

const stdioConfig = computed(() =>
  status.value
    ? JSON.stringify({ mcpServers: { "project-hub": { command: status.value.exe_path, args: ["--mcp"] } } }, null, 2)
    : ""
);

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
            <span>{{ t('mcpServer') }}<HelpTip :text="`${t('mcpDescription')}\n${t('mcpLocalOnly')}`" /></span>
            <span class="mcp-dot" :class="{ on: status?.running, bad: status?.enabled && !status?.running }" />
            <span class="text-[11px] font-normal text-muted">{{ statusLabel }}</span>
          </div>
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
        <button class="mcp-toggle" :aria-expanded="showSnippets" @click="showSnippets = !showSnippets">
          <ChevronDown :size="14" class="mcp-toggle-chevron" :class="{ open: showSnippets }" />
          {{ t('mcpConnect') }}
        </button>
        <Transition name="rise">
          <div v-if="showSnippets" class="space-y-3">
            <CodeSnippet :label="t('mcpClaudeGlobal')" :hint="t('mcpClaudeGlobalHint')" :text="claudeGlobalCommand" />
            <CodeSnippet :label="t('mcpClaudeLocal')" :hint="t('mcpClaudeLocalHint')" :text="claudeLocalCommand" />
            <CodeSnippet :label="t('mcpOtherAgents')" :hint="t('mcpOtherAgentsHint')" :text="stdioConfig" />
          </div>
        </Transition>
        <button class="btn btn-secondary btn-sm w-full" @click="showTools = true">
          <ListChecks :size="14" />{{ t('mcpToolsButton', { count: tools.length }) }}
        </button>
      </template>
    </div>
  </div>

  <McpToolsDialog :open="showTools" @close="showTools = false" />
</template>

<style scoped>
.mcp-dot {
  position: relative;
  flex: none;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--color-text-muted);
  transition: background-color var(--dur) var(--ease-out);
}

.mcp-dot.on {
  background: var(--color-success);
}

/* While the server runs, a ring keeps spreading out from the dot. */
.mcp-dot.on::after {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  background: var(--color-success);
  content: "";
  animation: mcp-pulse 2s var(--ease-out) infinite;
}

@keyframes mcp-pulse {
  from {
    opacity: 0.55;
    transform: scale(1);
  }
  to {
    opacity: 0;
    transform: scale(3.2);
  }
}

/* The global 1ms animations would make a looping ring flicker; keep a still halo instead. */
@media (prefers-reduced-motion: reduce) {
  .mcp-dot.on::after {
    animation: none;
    opacity: 0.22;
    transform: scale(1.9);
  }
}

.mcp-dot.bad {
  background: var(--color-error);
}

.mcp-toggle {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--color-muted);
  font-size: 12px;
}

.mcp-toggle:hover {
  color: var(--color-ink);
}

.mcp-toggle-chevron {
  transform: rotate(-90deg);
  transition: transform var(--dur-fast) ease;
}

.mcp-toggle-chevron.open {
  transform: rotate(0deg);
}
</style>
