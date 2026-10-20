import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";
import PanelResizer from "../components/PanelResizer.vue";

describe("PanelResizer", () => {
  function createWrapper(props = {}) {
    return mount(PanelResizer, {
      props: { side: "left", ...props },
    });
  }

  it("renders with col-resize cursor", () => {
    const wrapper = createWrapper();
    expect(wrapper.find(".cursor-col-resize").exists()).toBe(true);
  });

  it("tracks hover state via mouseenter/mouseleave", async () => {
    const wrapper = createWrapper();
    await wrapper.trigger("mouseenter");
    expect((wrapper.vm as any).isHovered).toBe(true);
    await wrapper.trigger("mouseleave");
    expect((wrapper.vm as any).isHovered).toBe(false);
  });

  it("starts drag mode on mousedown", async () => {
    const wrapper = createWrapper();
    await wrapper.trigger("mousedown", { clientX: 100, preventDefault: () => {} });
    expect((wrapper.vm as any).isDragging).toBe(true);
    document.dispatchEvent(new MouseEvent("mouseup"));
  });

  it("starts drag mode for right side too", async () => {
    const wrapper = createWrapper({ side: "right" });
    await wrapper.trigger("mousedown", { clientX: 100, preventDefault: () => {} });
    expect((wrapper.vm as any).isDragging).toBe(true);
    document.dispatchEvent(new MouseEvent("mouseup"));
  });

  it("emits incremental movement while dragging", async () => {
    const wrapper = createWrapper();
    await wrapper.trigger("mousedown", { clientX: 100, preventDefault: () => {} });

    document.dispatchEvent(new MouseEvent("mousemove", { clientX: 110 }));
    document.dispatchEvent(new MouseEvent("mousemove", { clientX: 115 }));
    document.dispatchEvent(new MouseEvent("mouseup"));

    expect(wrapper.emitted("resize")).toEqual([[10], [5]]);
  });

  it("inverts incremental movement for right side", async () => {
    const wrapper = createWrapper({ side: "right" });
    await wrapper.trigger("mousedown", { clientX: 100, preventDefault: () => {} });

    document.dispatchEvent(new MouseEvent("mousemove", { clientX: 110 }));
    document.dispatchEvent(new MouseEvent("mousemove", { clientX: 115 }));
    document.dispatchEvent(new MouseEvent("mouseup"));

    expect(wrapper.emitted("resize")).toEqual([[-10], [-5]]);
  });

  it("has inner visual indicator element", () => {
    const wrapper = createWrapper();
    const inner = wrapper.find("div.absolute");
    expect(inner.exists()).toBe(true);
  });
});
