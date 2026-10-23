<script setup lang="ts">
import { AppWindow, Code2, FolderOpen, SquareTerminal } from "lucide-vue-next";
import type { OpenerView } from "../../types";

/** The icon for a kind of program: terminal, editor, file manager or one the user added. */
defineProps<{ kind: OpenerView["kind"]; size?: number }>();
</script>

<template>
  <SquareTerminal v-if="kind === 'terminal'" :size="size ?? 14" />
  <FolderOpen v-else-if="kind === 'files'" :size="size ?? 14" />
  <Code2 v-else-if="kind === 'editor'" :size="size ?? 14" />
  <AppWindow v-else :size="size ?? 14" />
</template>
