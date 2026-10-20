<script setup lang="ts">
import { CheckCircle2, CircleAlert, Copy, Info, TriangleAlert, X } from "lucide-vue-next";
import { useToast, type Toast } from "../composables/useToast";
import { useI18n } from "../composables/useI18n";

const { toasts, dismiss, pause, resume } = useToast();
const { t } = useI18n();

const icons = {
  success: CheckCircle2,
  error: CircleAlert,
  warning: TriangleAlert,
  info: Info,
};

async function copy(toast: Toast) {
  try {
    await navigator.clipboard.writeText([toast.message, toast.detail].filter(Boolean).join("\n"));
  } catch {
    // Clipboard may be unavailable; nothing else to do.
  }
}

function runAction(toast: Toast) {
  toast.action?.run();
  dismiss(toast.id);
}
</script>

<template>
  <Teleport to="body">
    <div class="toast-stack" aria-live="polite">
      <TransitionGroup name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          class="toast"
          :class="`toast-${toast.kind}`"
          role="status"
          @mouseenter="pause(toast.id)"
          @mouseleave="resume(toast.id)"
        >
          <component :is="icons[toast.kind]" :size="16" class="toast-icon" />
          <div class="toast-body">
            <div class="toast-message">{{ toast.message }}</div>
            <div v-if="toast.detail" class="toast-detail">{{ toast.detail }}</div>
            <button v-if="toast.action" class="toast-action" @click="runAction(toast)">
              {{ toast.action.label }}
            </button>
          </div>
          <button
            v-if="toast.kind === 'error'"
            class="toast-close"
            :title="t('copy')"
            @click="copy(toast)"
          >
            <Copy :size="13" />
          </button>
          <button class="toast-close" :title="t('close')" @click="dismiss(toast.id)">
            <X :size="14" />
          </button>
          <span
            v-if="toast.duration > 0"
            class="toast-progress"
            :style="{ animationDuration: `${toast.duration}ms` }"
          />
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-stack {
  position: fixed;
  left: 50%;
  bottom: 22px;
  z-index: 10050;
  display: flex;
  flex-direction: column-reverse;
  align-items: center;
  gap: 8px;
  transform: translateX(-50%);
  pointer-events: none;
  width: min(460px, calc(100vw - 32px));
}

.toast {
  --accent: var(--color-info);
  position: relative;
  display: flex;
  align-items: flex-start;
  gap: 10px;
  width: 100%;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, var(--accent) 32%, var(--color-border));
  border-radius: 10px;
  background: color-mix(in srgb, var(--color-surface-overlay) 88%, var(--accent) 6%);
  color: var(--color-ink);
  box-shadow: 0 16px 40px rgb(0 0 0 / 0.28);
  padding: 10px 10px 10px 12px;
  font-size: 12.5px;
  line-height: 1.5;
  pointer-events: auto;
  backdrop-filter: blur(10px);
}

.toast-success { --accent: var(--color-success); }
.toast-error { --accent: var(--color-error); }
.toast-warning { --accent: var(--color-warning); }

.toast-icon {
  flex: none;
  margin-top: 1px;
  color: var(--accent);
}

.toast-body {
  flex: 1;
  min-width: 0;
}

.toast-message {
  word-break: break-word;
}

.toast-detail {
  margin-top: 2px;
  color: var(--color-muted);
  font-size: 12px;
  word-break: break-word;
  white-space: pre-line;
}

.toast-action {
  margin-top: 6px;
  border-radius: 6px;
  background: color-mix(in srgb, var(--accent) 18%, transparent);
  color: var(--color-ink);
  padding: 3px 10px;
  font-size: 12px;
  transition: background-color var(--dur-fast) var(--ease-out);
}

.toast-action:hover {
  background: color-mix(in srgb, var(--accent) 30%, transparent);
}

.toast-close {
  flex: none;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 6px;
  color: var(--color-muted);
  transition: background-color var(--dur-fast) var(--ease-out), color var(--dur-fast) var(--ease-out);
}

.toast-close:hover {
  background: var(--color-surface-raised);
  color: var(--color-ink);
}

.toast-progress {
  position: absolute;
  left: 0;
  bottom: 0;
  height: 2px;
  width: 100%;
  background: color-mix(in srgb, var(--accent) 55%, transparent);
  transform-origin: left;
  animation: toast-countdown linear forwards;
}

.toast:hover .toast-progress {
  animation-play-state: paused;
}

@keyframes toast-countdown {
  from { transform: scaleX(1); }
  to { transform: scaleX(0); }
}

.toast-enter-active {
  transition: opacity var(--dur) var(--ease-out), transform var(--dur-slow) var(--ease-spring);
}

.toast-leave-active {
  transition: opacity var(--dur-fast) ease, transform var(--dur-fast) ease;
}

.toast-move {
  transition: transform var(--dur) var(--ease-out);
}

.toast-enter-from {
  opacity: 0;
  transform: translateY(14px) scale(0.96);
}

.toast-leave-to {
  opacity: 0;
  transform: scale(0.96);
}
</style>
