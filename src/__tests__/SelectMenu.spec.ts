import { mount } from "@vue/test-utils";
import { describe, it, expect, afterEach, vi } from "vitest";
import { nextTick } from "vue";
import SelectMenu from "../components/SelectMenu.vue";
import { choosePanelSide } from "../utils/popupPlacement";

let unmount: (() => void) | null = null;

const options = Array.from({ length: 30 }, (_, i) => ({ value: i, label: `Option ${i}` }));

function rect(top: number): DOMRect {
  return { top, bottom: top + 30, left: 20, right: 220, width: 200, height: 30, x: 20, y: top, toJSON() {} } as DOMRect;
}

async function openMenu() {
  const wrapper = mount(SelectMenu, { props: { options, modelValue: 0 }, attachTo: document.body });
  unmount = () => wrapper.unmount();
  const trigger = wrapper.get("button").element;
  let top = 100;
  vi.spyOn(trigger, "getBoundingClientRect").mockImplementation(() => rect(top));
  await wrapper.get("button").trigger("click");
  await nextTick();
  const panel = document.querySelector<HTMLElement>(".select-panel")!;
  const scroll = async (target: EventTarget, newTop = top) => {
    top = newTop;
    target.dispatchEvent(new Event("scroll"));
    await new Promise((resolve) => requestAnimationFrame(resolve));
    await nextTick();
  };
  return { wrapper, panel, scroll, isOpen: () => !!document.querySelector(".select-panel") };
}

describe("SelectMenu while scrolling", () => {
  afterEach(() => {
    unmount?.();
    unmount = null;
    vi.restoreAllMocks();
  });

  it("stays open when its own list is scrolled", async () => {
    const { panel, scroll, isOpen } = await openMenu();
    await scroll(panel);
    expect(isOpen()).toBe(true);
  });

  it("follows its button when the page scrolls", async () => {
    const { panel, scroll, isOpen } = await openMenu();
    expect(panel.style.top).toBe("134px");
    await scroll(document, 60);
    expect(isOpen()).toBe(true);
    expect(panel.style.top).toBe("94px");
  });

  it("closes once its button is scrolled out of sight", async () => {
    const { scroll, isOpen } = await openMenu();
    await scroll(document, -200);
    expect(isOpen()).toBe(false);
  });
});

describe("choosePanelSide", () => {
  it("opens downward when there is room, upward near the bottom of the window", () => {
    const height = window.innerHeight;
    expect(choosePanelSide(rect(100), 280).upward).toBe(false);
    expect(choosePanelSide(rect(height - 60), 280).upward).toBe(true);
  });
});
