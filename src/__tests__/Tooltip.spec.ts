import { mount } from "@vue/test-utils";
import { afterEach, describe, expect, it } from "vitest";
import { nextTick } from "vue";
import Tooltip from "../components/Tooltip.vue";

afterEach(() => {
  document.body.innerHTML = "";
});

function setElementRect(element: Element, rect: Partial<DOMRect>) {
  element.getBoundingClientRect = () => ({
    x: rect.left ?? 0,
    y: rect.top ?? 0,
    width: rect.width ?? 0,
    height: rect.height ?? 0,
    top: rect.top ?? 0,
    right: rect.right ?? 0,
    bottom: rect.bottom ?? 0,
    left: rect.left ?? 0,
    toJSON: () => ({}),
  } as DOMRect);
}

describe("Tooltip", () => {
  it("flips below the trigger when there is no room above", async () => {
    const wrapper = mount(Tooltip, {
      props: { text: "Add project" },
      slots: { default: "<button>Add</button>" },
    });

    const trigger = wrapper.find("span");
    setElementRect(trigger.element, {
      left: 20,
      top: 2,
      right: 52,
      bottom: 34,
      width: 32,
      height: 32,
    });

    await trigger.trigger("mouseenter");
    await nextTick();

    const tooltip = document.body.querySelector(".tooltip-floating") as HTMLElement | null;
    expect(tooltip).not.toBeNull();
    expect(tooltip?.style.top).toBe("40px");
    expect(tooltip?.style.transform).toBe("translate(-50%, 0)");
  });
});
