import { onBeforeUnmount, watch, type Ref } from "vue";
import { isScrolledIntoView } from "../utils/popupPlacement";

export interface FollowAnchorOptions {
  /** The element the popup hangs from (a dropdown's button). */
  anchor: Ref<HTMLElement | null>;
  /** Whether the popup is shown; nothing is watched while it is closed. */
  isOpen: () => boolean;
  /** Called (at most once per frame) when the page scrolls or resizes, to move the popup along. */
  onMove: () => void;
  /** Called when the anchor has scrolled out of sight, where the popup no longer makes sense. */
  onHidden: () => void;
}

/**
 * Keeps a floating popup attached to its anchor while what is behind it scrolls, instead of
 * closing at the first turn of the wheel; it closes once the anchor itself is scrolled away.
 */
export function useFollowAnchor(options: FollowAnchorOptions) {
  let frame = 0;

  function update() {
    frame = 0;
    if (!options.isOpen()) return;
    const anchor = options.anchor.value;
    if (!anchor || !isScrolledIntoView(anchor)) options.onHidden();
    else options.onMove();
  }

  function schedule() {
    if (!frame) frame = requestAnimationFrame(update);
  }

  function stop() {
    document.removeEventListener("scroll", schedule, true);
    window.removeEventListener("resize", schedule);
    if (frame) cancelAnimationFrame(frame);
    frame = 0;
  }

  watch(options.isOpen, (open) => {
    stop();
    if (!open) return;
    document.addEventListener("scroll", schedule, true);
    window.addEventListener("resize", schedule);
  });

  onBeforeUnmount(stop);
}
