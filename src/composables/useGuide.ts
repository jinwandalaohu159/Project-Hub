import { nextTick, ref, type Ref } from "vue";
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
/** True while the guide morphs between layouts; its own open / close animations stand aside. */
const switchingLayout = ref(false);
let layout: Ref<GuideLayout> | null = null;

/** The user guide's shared state: whether it is shown, where, and at which chapter. */
export function useGuide() {
  layout ??= usePersistentRef<GuideLayout>("guide-layout", "center");
  const currentLayout = layout;

  /** Opens the guide, at `at` when given (scrolling there if it is already open). */
  function show(at?: GuideSectionId) {
    if (at) {
      section.value = at;
      jump.value = { id: at };
    }
    open.value = true;
  }

  function close() {
    open.value = false;
  }

  function toggle() {
    open.value = !open.value;
  }

  /** Moves the guide between the middle of the window and its right edge, morphing in place. */
  function toggleLayout() {
    switchingLayout.value = true;
    morphLayout(() => {
      currentLayout.value = currentLayout.value === "center" ? "docked" : "center";
      void nextTick(() => (switchingLayout.value = false));
    });
  }

  return { open, section, jump, layout: currentLayout, switchingLayout, show, close, toggle, toggleLayout };
}
