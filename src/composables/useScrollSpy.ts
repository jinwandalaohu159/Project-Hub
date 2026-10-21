import { onBeforeUnmount, onMounted, type Ref } from "vue";

/** How far below the top of the scroller a heading counts as the one being read. */
const READING_LINE = 32;

/**
 * Keeps `active` on the `[data-section]` element being read inside `scroller`, and scrolls to one
 * on request. While a requested scroll runs, the sections it passes do not take over `active`.
 */
export function useScrollSpy(scroller: Ref<HTMLElement | null>, active: Ref<string>) {
  let target: string | null = null;
  let frame = 0;

  function sections() {
    return Array.from(scroller.value?.querySelectorAll<HTMLElement>("[data-section]") ?? []);
  }

  function atBottom(el: HTMLElement) {
    return el.scrollTop + el.clientHeight >= el.scrollHeight - 2;
  }

  function update() {
    frame = 0;
    const el = scroller.value;
    if (!el) return;
    const all = sections();
    if (target) {
      const goal = all.find((section) => section.dataset.section === target);
      if (!goal || Math.abs(goal.offsetTop - el.scrollTop) <= 2 || atBottom(el)) target = null;
      return;
    }
    const line = el.scrollTop + READING_LINE;
    let current = all[0]?.dataset.section;
    for (const section of all) if (section.offsetTop <= line) current = section.dataset.section;
    if (atBottom(el)) current = all[all.length - 1]?.dataset.section ?? current;
    if (current) active.value = current;
  }

  function onScroll() {
    if (!frame) frame = requestAnimationFrame(update);
  }

  function scrollTo(id: string, smooth = true) {
    const el = scroller.value;
    const section = sections().find((candidate) => candidate.dataset.section === id);
    if (!el || !section) return;
    active.value = id;
    target = smooth ? id : null;
    el.scrollTo?.({ top: section.offsetTop, behavior: smooth ? "smooth" : "auto" });
  }

  onMounted(() => scroller.value?.addEventListener("scroll", onScroll, { passive: true }));
  onBeforeUnmount(() => {
    scroller.value?.removeEventListener("scroll", onScroll);
    if (frame) cancelAnimationFrame(frame);
  });

  return { scrollTo };
}
