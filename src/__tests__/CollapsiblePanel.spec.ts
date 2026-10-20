import { mount } from "@vue/test-utils";
import { describe, it, expect } from "vitest";
import CollapsiblePanel from "../components/CollapsiblePanel.vue";

describe("CollapsiblePanel", () => {
  it("renders title and collapse button", () => {
    const wrapper = mount(CollapsiblePanel, {
      props: { collapsed: false, title: "Test Panel", side: "left" },
      slots: { default: '<div data-testid="panel-content">Content</div>' },
    });

    expect(wrapper.text()).toContain("Test Panel");
    expect(wrapper.find('[data-testid="collapse-btn"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="panel-content"]').exists()).toBe(true);
  });

  it("hides content when collapsed", () => {
    const wrapper = mount(CollapsiblePanel, {
      props: { collapsed: true, title: "Test Panel", side: "left" },
      slots: { default: '<div data-testid="panel-content">Content</div>' },
    });

    const body = wrapper.find('.flex-1.min-h-0');
    expect(body.exists()).toBe(true);
    expect(body.attributes('style')).toContain('display: none');
  });

  it("emits update:collapsed when toggle button clicked", async () => {
    const wrapper = mount(CollapsiblePanel, {
      props: { collapsed: false, title: "Test Panel", side: "left" },
      slots: { default: "<div>Content</div>" },
    });

    await wrapper.find('[data-testid="collapse-btn"]').trigger("click");
    expect(wrapper.emitted("update:collapsed")).toEqual([[true]]);
  });

  it("emits update:collapsed when title bar clicked", async () => {
    const wrapper = mount(CollapsiblePanel, {
      props: { collapsed: false, title: "Test Panel", side: "left" },
      slots: { default: "<div>Content</div>" },
    });

    await wrapper.find('[data-testid="panel-title-bar"]').trigger("click");
    expect(wrapper.emitted("update:collapsed")).toEqual([[true]]);
  });

  it("renders collapse button when collapsed on left side", () => {
    const wrapper = mount(CollapsiblePanel, {
      props: { collapsed: true, title: "Test", side: "left" },
      slots: { default: "<div />" },
    });

    expect(wrapper.find('[data-testid="collapse-btn"]').exists()).toBe(true);
  });

  it("renders collapse button when collapsed on right side", () => {
    const wrapper = mount(CollapsiblePanel, {
      props: { collapsed: true, title: "Test", side: "right" },
      slots: { default: "<div />" },
    });

    expect(wrapper.find('[data-testid="collapse-btn"]').exists()).toBe(true);
  });

  it("renders collapse button when expanded", () => {
    const wrapper = mount(CollapsiblePanel, {
      props: { collapsed: false, title: "Test", side: "left" },
      slots: { default: "<div />" },
    });

    expect(wrapper.find('[data-testid="collapse-btn"]').exists()).toBe(true);
  });
});