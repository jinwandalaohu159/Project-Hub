import { ref } from "vue";

export type ToastKind = "success" | "error" | "info" | "warning";

export interface ToastAction {
  label: string;
  run: () => void;
}

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
  detail?: string;
  action?: ToastAction;
  duration: number;
}

interface ToastOptions {
  detail?: string;
  action?: ToastAction;
  /** Milliseconds before auto-dismiss; 0 keeps it until closed. */
  duration?: number;
}

const DEFAULT_DURATION: Record<ToastKind, number> = {
  success: 2800,
  info: 3600,
  warning: 5000,
  error: 6500,
};
const MAX_TOASTS = 4;

const toasts = ref<Toast[]>([]);
const timers = new Map<number, { handle: ReturnType<typeof setTimeout>; endsAt: number; remaining: number }>();
let nextId = 1;

function clearTimer(id: number) {
  const timer = timers.get(id);
  if (timer) clearTimeout(timer.handle);
  timers.delete(id);
}

function schedule(id: number, ms: number) {
  clearTimer(id);
  if (ms <= 0) return;
  timers.set(id, {
    handle: setTimeout(() => dismiss(id), ms),
    endsAt: Date.now() + ms,
    remaining: ms,
  });
}

function dismiss(id: number) {
  clearTimer(id);
  toasts.value = toasts.value.filter((toast) => toast.id !== id);
}

function push(kind: ToastKind, message: string, options: ToastOptions = {}) {
  const duration = options.duration ?? DEFAULT_DURATION[kind];
  // Repeating the same message refreshes it instead of stacking copies.
  const existing = toasts.value.find((toast) => toast.kind === kind && toast.message === message);
  if (existing) {
    existing.detail = options.detail;
    existing.action = options.action;
    schedule(existing.id, duration);
    return existing.id;
  }

  const toast: Toast = { id: nextId++, kind, message, duration, ...options };
  toasts.value = [...toasts.value, toast];
  while (toasts.value.length > MAX_TOASTS) {
    dismiss(toasts.value[0].id);
  }
  schedule(toast.id, duration);
  return toast.id;
}

/** Hovering a toast pauses its countdown so long messages can be read. */
function pause(id: number) {
  const timer = timers.get(id);
  if (!timer) return;
  clearTimeout(timer.handle);
  timers.set(id, { ...timer, remaining: Math.max(0, timer.endsAt - Date.now()) });
}

function resume(id: number) {
  const timer = timers.get(id);
  if (!timer) return;
  schedule(id, Math.max(timer.remaining, 1200));
}

export function useToast() {
  return {
    toasts,
    push,
    dismiss,
    pause,
    resume,
    success: (message: string, options?: ToastOptions) => push("success", message, options),
    info: (message: string, options?: ToastOptions) => push("info", message, options),
    warning: (message: string, options?: ToastOptions) => push("warning", message, options),
    error: (message: string, options?: ToastOptions) => push("error", message, options),
  };
}
