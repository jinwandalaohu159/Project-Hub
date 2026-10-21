import { ref, type Ref } from "vue";
import { usePersistentRef } from "./usePersistentRef";
import { morphLayout } from "../utils/viewTransition";
import type { GuideSectionId } from "../utils/guide";

/** Centered over a blurred app, or docked beside it to read while working. */
export type GuideLayout = "center" | "docked";

const open = ref(false);
/** The chapter being read; reopening the guide continues there. */
const section = ref<GuideSectionId>("welcome");
/** Set when something asks for a particular chapter, so an open guide scrolls to it. */
const jump = ref<{ id: GuideSectionId } | null>(null);
let layout: Ref<GuideLayout> | null = null;

/**
 * The user guide's shared state: whether it is shown, where, and at which chapter. Showing,
 * hiding and moving it are layout morphs, so it eases in and out of place.
 */
export function useGuide() {
  layout ??= usePersistentRef<GuideLayout>("guide-layout", "center");
  const currentLayout = layout;

  function setOpen(value: boolean) {
    if (open.value !== value) morphLayout(() => (open.value = value));
  }

  /** Opens the guide, at `at` when given (scrolling there if it is already open). */
  function show(at?: GuideSectionId) {
    if (at) {
      section.value = at;
      jump.value = { id: at };
    }
    setOpen(true);
  }

  function close() {
    setOpen(false);
  }

  function toggle() {
    setOpen(!open.value);
  }

  /** Moves the guide between the middle of the window and its right edge. */
  function toggleLayout() {
    morphLayout(() => {
      currentLayout.value = currentLayout.value === "center" ? "docked" : "center";
    });
  }

  return { open, section, jump, layout: currentLayout, show, close, toggle, toggleLayout };
}
