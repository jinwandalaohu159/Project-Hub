<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { BookOpen, PanelRight, AppWindow, X } from "lucide-vue-next";
import GuideNav from "./GuideNav.vue";
import GuideArticle from "./GuideArticle.vue";
import SelectMenu from "../SelectMenu.vue";
import Tooltip from "../Tooltip.vue";
import { useI18n } from "../../composables/useI18n";
import { useGuide, type GuideLayout } from "../../composables/useGuide";
import { useScrollSpy } from "../../composables/useScrollSpy";
import { guideFor, type GuideAction, type GuideSectionId } from "../../utils/guide";

/** The guide itself: header, table of contents and chapters, the same wherever it is shown. */
const props = defineProps<{ layout: GuideLayout }>();
const emit = defineEmits<{ action: [action: GuideAction] }>();

const { t, locale } = useI18n();
const { section, jump, close, toggleLayout } = useGuide();

const guide = computed(() => guideFor(locale.value));
const rootRef = ref<HTMLElement | null>(null);
const scrollerRef = ref<HTMLElement | null>(null);
const { scrollTo } = useScrollSpy(scrollerRef, section);

/** The narrow table of contents (a dropdown) jumps like the wide one. */
const jumpOptions = computed(() =>
  guide.value.sections.map((item, index) => ({ value: item.id, label: `${index + 1}. ${item.title}` })),
);
const jumpTo = computed({
  get: () => section.value,
  set: (id: GuideSectionId | "") => {
    if (id) scrollTo(id);
  },
});

// Something outside asked for a chapter while the guide is open.
watch(jump, (request) => {
  if (request) scrollTo(request.id);
});

let focusedBefore: Element | null = null;

onMounted(() => {
  // Continue where the reader left off.
  scrollTo(section.value, false);
  if (props.layout === "center") {
    focusedBefore = document.activeElement;
    rootRef.value?.focus({ preventScroll: true });
  }
});

onBeforeUnmount(() => {
  if (focusedBefore instanceof HTMLElement) focusedBefore.focus({ preventScroll: true });
});
</script>

<template>
  <div ref="rootRef" class="guide-reader" :class="`guide-${layout}`" role="dialog" :aria-label="guide.title" tabindex="-1">
    <header class="guide-header">
      <span class="guide-header-icon"><BookOpen :size="16" /></span>
      <div class="min-w-0 flex-1">
        <h1 class="guide-title">{{ guide.title }}</h1>
        <p class="guide-subtitle">{{ guide.subtitle }}</p>
      </div>
      <Tooltip :text="layout === 'center' ? t('guideDock') : t('guideCenter')" position="bottom">
        <button type="button" class="guide-header-btn" @click="toggleLayout">
          <PanelRight v-if="layout === 'center'" :size="16" />
          <AppWindow v-else :size="16" />
        </button>
      </Tooltip>
      <Tooltip :text="t('guideClose')" position="bottom">
        <button type="button" class="guide-header-btn" @click="close">
          <X :size="16" />
        </button>
      </Tooltip>
    </header>

    <div class="guide-body">
      <aside class="guide-toc">
        <div class="guide-toc-label">{{ t('guideContents') }}</div>
        <GuideNav :sections="guide.sections" :active="section" @select="scrollTo" />
      </aside>
      <div class="guide-toc-compact">
        <SelectMenu v-model="jumpTo" :options="jumpOptions" size="sm" />
      </div>
      <div ref="scrollerRef" class="guide-scroll">
        <GuideArticle :sections="guide.sections" @select="scrollTo" @action="emit('action', $event)" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.guide-reader {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: hidden;
  background: var(--color-surface);
  outline: none;
  container: guide / inline-size;
  view-transition-name: guide-reader;
}

.guide-center {
  width: min(980px, 100%);
  height: min(86vh, 800px);
  border: 1px solid var(--color-border);
  border-radius: 14px;
  box-shadow: 0 24px 64px rgb(0 0 0 / 0.38), 0 1px 0 rgb(255 255 255 / 0.04) inset;
}

.guide-docked {
  height: 100%;
}

.guide-header {
  display: flex;
  flex: none;
  align-items: center;
  gap: 10px;
  padding: 14px 14px 12px 18px;
  border-bottom: 1px solid var(--color-border-subtle);
}

.guide-header-icon {
  display: flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: 8px;
  background: color-mix(in srgb, var(--color-primary) 15%, transparent);
  color: var(--color-primary);
}

.guide-title {
  color: var(--color-ink);
  font-size: 15px;
  font-weight: 600;
}

.guide-subtitle {
  overflow: hidden;
  color: var(--color-muted);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.guide-header-btn {
  display: flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border-radius: 7px;
  color: var(--color-muted);
  transition: background-color var(--dur-fast) ease, color var(--dur-fast) ease;
}

.guide-header-btn:hover {
  background: var(--color-surface-raised);
  color: var(--color-ink);
}

.guide-body {
  display: flex;
  min-height: 0;
  flex: 1;
}

.guide-toc {
  flex: none;
  width: 220px;
  overflow-y: auto;
  border-right: 1px solid var(--color-border-subtle);
  padding: 16px 10px;
}

.guide-toc-label {
  margin: 0 10px 8px;
  color: var(--color-muted);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.05em;
  text-transform: uppercase;
}

.guide-toc-compact {
  display: none;
  flex: none;
  padding: 10px 18px;
  border-bottom: 1px solid var(--color-border-subtle);
}

.guide-scroll {
  position: relative;
  min-width: 0;
  flex: 1;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: 0 32px;
}

/* Too narrow for a side table of contents: it becomes a dropdown above the chapters. */
@container guide (max-width: 640px) {
  .guide-body {
    flex-direction: column;
  }

  .guide-toc {
    display: none;
  }

  .guide-toc-compact {
    display: block;
  }

  .guide-scroll {
    padding: 0 20px;
  }
}
</style>
