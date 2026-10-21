import { onBeforeUnmount, onMounted } from "vue";

export interface DismissOptions {
  /** Whether the popup is currently shown; nothing is done while it is closed. */
  isOpen: () => boolean;
  /** True for clicks that belong to the popup itself (or its trigger). */
  isInside: (target: Element) => boolean;
  onDismiss: () => void;
  /** Return false to keep the popup open for now (e.g. while it is busy). */
  canDismiss?: () => boolean;
  /** Also close when something outside the popup scrolls (context menus). */
  closeOnScroll?: boolean;
  /** Also close when the window loses focus (floating menus). */
  closeOnBlur?: boolean;
}

const MODAL_SELECTOR = ".modal-backdrop";

/**
 * Closes a popup like a dropdown: clicking anywhere else or pressing Escape dismisses it.
 * Clicks inside a dialog opened from the popup (a confirmation, for example) do not count.
 */
export function useDismiss(options: DismissOptions) {
  const active = () => options.isOpen() && (options.canDismiss?.() ?? true);

  function onPointerDown(event: PointerEvent) {
    if (!active()) return;
    const target = event.target as Element | null;
    if (!target || options.isInside(target) || target.closest(MODAL_SELECTOR)) return;
    options.onDismiss();
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key !== "Escape" || !active()) return;
    // Escape belongs to the dialog when one is open on top.
    if (document.querySelector(MODAL_SELECTOR)) return;
    options.onDismiss();
  }

  function onScroll(event: Event) {
    // Scrolling the popup's own list is using it, not leaving it.
    const target = event.target;
    if (target instanceof Element && options.isInside(target)) return;
    if (active()) options.onDismiss();
  }

  function onBlur() {
    if (active()) options.onDismiss();
  }

  onMounted(() => {
    document.addEventListener("pointerdown", onPointerDown, true);
    document.addEventListener("keydown", onKeydown);
    if (options.closeOnScroll) document.addEventListener("scroll", onScroll, true);
    if (options.closeOnBlur) window.addEventListener("blur", onBlur);
  });

  onBeforeUnmount(() => {
    document.removeEventListener("pointerdown", onPointerDown, true);
    document.removeEventListener("keydown", onKeydown);
    document.removeEventListener("scroll", onScroll, true);
    window.removeEventListener("blur", onBlur);
  });
}
