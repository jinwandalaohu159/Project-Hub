import { ref } from "vue";

export interface ConfirmOptions {
  title: string;
  message?: string;
  /** Extra lines rendered as a highlighted note under the message. */
  note?: string;
  confirmText?: string;
  cancelText?: string;
  danger?: boolean;
}

interface PendingConfirm extends ConfirmOptions {
  resolve: (value: boolean) => void;
}

const pending = ref<PendingConfirm | null>(null);

export function useConfirm() {
  function confirm(options: ConfirmOptions): Promise<boolean> {
    pending.value?.resolve(false);
    return new Promise((resolve) => {
      pending.value = { ...options, resolve };
    });
  }

  function settle(value: boolean) {
    const current = pending.value;
    pending.value = null;
    current?.resolve(value);
  }

  return { pending, confirm, settle };
}
