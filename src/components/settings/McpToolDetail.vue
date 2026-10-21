<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Loader2, Play } from "lucide-vue-next";
import { useI18n } from "../../composables/useI18n";
import { useProjectStore } from "../../composables/useProjectStore";
import { isReadOnly, useMcpTools } from "../../composables/useMcpTools";
import { errorText } from "../../utils/errors";
import { EXAMPLE_PROJECT, MCP_TOOL_DOCS } from "../../utils/mcpToolDocs";
import type { McpTool } from "../../types";
import SelectMenu from "../SelectMenu.vue";
import CodeSnippet from "./CodeSnippet.vue";
import McpToolBadge from "./McpToolBadge.vue";

/** One MCP tool in full: what it does, its parameters, an example call and what it returns. */
const props = defineProps<{ tool: McpTool }>();

const { t } = useI18n();
const { projects, selectedProject } = useProjectStore();
const { tryTool } = useMcpTools();

const doc = computed(() => MCP_TOOL_DOCS[props.tool.name]);
const readOnly = computed(() => isReadOnly(props.tool));
const summary = computed(() => (doc.value ? t(doc.value.summary) : props.tool.description));

const params = computed(() => {
  const required = new Set(props.tool.inputSchema.required ?? []);
  return Object.entries(props.tool.inputSchema.properties ?? {}).map(([name, schema]) => {
    const docKey = doc.value?.params?.[name];
    return {
      name,
      type: schema.type === "array" ? `${schema.items?.type ?? "any"}[]` : (schema.type ?? "any"),
      required: required.has(name),
      description: docKey ? t(docKey) : (schema.description ?? ""),
    };
  });
});

// Examples and "try it" use a real project, the one open in the app by default.
const tryProjectId = ref(selectedProject.value?.id ?? projects.value[0]?.id ?? "");
const tryProject = computed(() => projects.value.find((project) => project.id === tryProjectId.value) ?? null);
const projectOptions = computed(() => projects.value.map((project) => ({ value: project.id, label: project.name })));

function exampleArgs(project: string) {
  return doc.value ? doc.value.example(project, t) : {};
}

const exampleCall = computed(() =>
  JSON.stringify({ name: props.tool.name, arguments: exampleArgs(tryProject.value?.path ?? EXAMPLE_PROJECT) }, null, 2),
);

const running = ref(false);
const result = ref<{ text: string; failed: boolean } | null>(null);

watch([() => props.tool.name, tryProjectId], () => {
  result.value = null;
});

async function run() {
  const project = tryProject.value;
  if (running.value || (!project && params.value.some((param) => param.name === "project"))) return;
  running.value = true;
  try {
    result.value = { text: await tryTool(props.tool.name, exampleArgs(project?.path ?? "")), failed: false };
  } catch (error) {
    result.value = { text: errorText(error, t), failed: true };
  } finally {
    running.value = false;
  }
}
</script>

<template>
  <div class="space-y-4">
    <div>
      <div class="flex items-center justify-between gap-2">
        <span class="tool-title">{{ tool.name }}</span>
        <McpToolBadge :read-only="readOnly" />
      </div>
      <p class="mt-1.5 text-xs leading-5 text-muted">{{ summary }}</p>
    </div>

    <section>
      <h4 class="detail-heading">{{ t('mcpToolParams') }}</h4>
      <p v-if="params.length === 0" class="text-xs text-muted">{{ t('mcpToolNoParams') }}</p>
      <div v-else class="param-list">
        <div v-for="param in params" :key="param.name" class="param-row">
          <div class="flex flex-wrap items-center gap-1.5">
            <span class="param-name">{{ param.name }}</span>
            <span class="param-chip">{{ param.type }}</span>
            <span class="param-chip" :class="{ required: param.required }">
              {{ param.required ? t('mcpToolRequired') : t('mcpToolOptional') }}
            </span>
          </div>
          <p class="mt-1 text-xs leading-5 text-muted">{{ param.description }}</p>
        </div>
      </div>
    </section>

    <section>
      <h4 class="detail-heading">{{ t('mcpToolExample') }}</h4>
      <CodeSnippet :label="t('mcpToolExample')" :text="exampleCall" />
    </section>

    <section>
      <h4 class="detail-heading">{{ t('mcpToolReturns') }}</h4>
      <p class="mb-2 text-xs leading-5 text-muted">{{ doc ? t(doc.returns) : '' }}</p>

      <template v-if="readOnly">
        <p v-if="projects.length === 0" class="text-xs text-muted">{{ t('mcpToolTryNoProjects') }}</p>
        <div v-else class="try-row">
          <div class="min-w-0 flex-1">
            <SelectMenu v-model="tryProjectId" :options="projectOptions" size="sm" />
          </div>
          <button class="btn btn-secondary btn-sm shrink-0" :disabled="running" @click="run">
            <Loader2 v-if="running" :size="13" class="spin" /><Play v-else :size="13" />
            {{ t('mcpToolTry') }}
          </button>
        </div>
        <p v-if="projects.length > 0" class="mt-1.5 text-[11px] leading-4 text-muted">{{ t('mcpToolTryHint') }}</p>
        <Transition name="rise">
          <div v-if="result" class="mt-2" :class="{ 'try-failed': result.failed }">
            <CodeSnippet :label="t('mcpToolReturns')" :text="result.text" max-height="260px" />
          </div>
        </Transition>
      </template>
      <template v-else>
        <p class="mb-2 text-[11px] leading-4 text-muted">{{ t('mcpToolNoTry') }}</p>
        <CodeSnippet v-if="doc?.sampleResult" :label="t('mcpToolSample')" :text="doc.sampleResult(t)" />
      </template>
    </section>
  </div>
</template>

<style scoped>
.tool-title {
  color: var(--color-ink);
  font-family: "Cascadia Mono", Consolas, "SF Mono", Menlo, monospace;
  font-size: 14px;
  font-weight: 600;
}

.detail-heading {
  margin-bottom: 6px;
  color: var(--color-muted);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.param-list {
  overflow: hidden;
  border: 1px solid var(--color-border-subtle);
  border-radius: 8px;
}

.param-row {
  background: var(--color-canvas);
  padding: 8px 10px;
}

.param-row + .param-row {
  border-top: 1px solid var(--color-border-subtle);
}

.param-name {
  color: var(--color-ink);
  font-family: "Cascadia Mono", Consolas, "SF Mono", Menlo, monospace;
  font-size: 12px;
}

.param-chip {
  border-radius: 4px;
  background: var(--color-surface-raised);
  padding: 0 6px;
  color: var(--color-muted);
  font-size: 10.5px;
  line-height: 17px;
}

.param-chip.required {
  background: color-mix(in srgb, var(--color-primary) 16%, transparent);
  color: var(--color-primary);
}

.try-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.try-failed :deep(code) {
  color: var(--color-error);
}
</style>
