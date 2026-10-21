<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "../../composables/useI18n";
import type { EditorPalette, TokenRole } from "../../utils/editorThemes";

/**
 * A few lines of code in a theme's colors, colored the way the code viewer's tokenizer sees them
 * (names are plain identifiers there), so what you pick is what the viewer shows.
 */
const props = defineProps<{ palette: EditorPalette }>();
const { t } = useI18n();

type Piece = [TokenRole | null, string];

/** The line drawn as the cursor's line. */
const CURRENT_LINE = 2;

const lines = computed<Piece[][]>(() => [
  [["comment", `// ${t("codeThemePreviewComment")}`]],
  [
    ["keyword", "const"], [null, " "], ["identifier", "version"], [null, " "], ["delimiter", "="], [null, " "],
    ["keyword", "new"], [null, " "], ["type", "Version"], ["delimiter", "("], ["string", '"main"'],
    ["delimiter", ","], [null, " "], ["number", "3"], ["delimiter", ");"],
  ],
  [
    ["keyword", "function"], [null, " "], ["identifier", "save"], ["delimiter", "("], ["identifier", "title"],
    ["delimiter", ")"], [null, " "], ["delimiter", "{"],
  ],
  [
    [null, "  "], ["keyword", "return"], [null, " "], ["identifier", "version"], ["delimiter", "."],
    ["identifier", "commit"], ["delimiter", "("], ["identifier", "title"], ["delimiter", ");"],
  ],
  [["delimiter", "}"]],
]);

function color(role: TokenRole | null) {
  return (role && props.palette.tokens[role]) || props.palette.foreground;
}
</script>

<template>
  <div class="preview" :style="{ background: palette.background, color: palette.foreground }">
    <div
      v-for="(line, index) in lines"
      :key="index"
      class="preview-line"
      :style="index === CURRENT_LINE ? { background: palette.lineHighlight } : undefined"
    >
      <span
        class="preview-number"
        :style="{ color: index === CURRENT_LINE ? palette.lineNumberActive : palette.lineNumber }"
      >{{ index + 1 }}</span>
      <span class="preview-code"><span
          v-for="([role, text], piece) in line"
          :key="piece"
          :style="{ color: color(role), fontStyle: role === 'comment' && palette.italicComments ? 'italic' : undefined }"
        >{{ text }}</span></span>
    </div>
  </div>
</template>

<style scoped>
.preview {
  overflow: hidden;
  border: 1px solid var(--color-border);
  border-radius: 6px;
  padding: 8px 0;
  font-family: ui-monospace, "Cascadia Code", Consolas, "SF Mono", Menlo, monospace;
  font-size: 12px;
  line-height: 19px;
  transition: background-color var(--dur) var(--ease-out), color var(--dur) var(--ease-out);
}

.preview-line {
  display: flex;
  white-space: pre;
}

.preview-number {
  flex: none;
  width: 30px;
  padding-right: 12px;
  text-align: right;
  user-select: none;
}

.preview-code {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

.preview-code span {
  transition: color var(--dur) var(--ease-out);
}
</style>
