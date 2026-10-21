import { nextTick } from "vue";

type ViewTransitionDocument = Document & {
  startViewTransition?: (update: () => Promise<void>) => { finished: Promise<void> };
};

/** Set on <html> while a layout morph runs; size transitions pause so they don't fight it. */
const MORPHING_CLASS = "layout-morphing";

/**
 * Applies a layout change as a view transition: the browser snapshots the old layout, Vue renders
 * the new one, and named panels morph from their old place and size to the new. Where view
 * transitions are unavailable (or reduced motion is asked for) the change simply happens.
 */
export function morphLayout(update: () => void) {
  const doc = typeof document === "undefined" ? null : (document as ViewTransitionDocument);
  const reducedMotion =
    typeof window !== "undefined" && window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
  if (!doc?.startViewTransition || reducedMotion) {
    update();
    return;
  }

  const root = doc.documentElement;
  root.classList.add(MORPHING_CLASS);
  const transition = doc.startViewTransition(async () => {
    update();
    await nextTick();
  });
  transition.finished.finally(() => root.classList.remove(MORPHING_CLASS));
}
