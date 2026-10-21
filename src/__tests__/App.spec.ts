import { mount } from "@vue/test-utils";
import { describe, it, expect } from "vitest";
import App from "../App.vue";

const collapsiblePanelStub = {
  template: '<div data-testid="collapsible-panel"><slot /><slot name="title-actions" /></div>',
  props: ['collapsed', 'title', 'side'],
}

const panelResizerStub = { template: '<div data-testid="panel-resizer" />' }

describe("App layout", () => {
  it("renders three collapsible panels in horizontal layout", () => {
    const wrapper = mount(App, {
      global: {
        stubs: {
          ProjectPanel: { template: '<div data-testid="project-panel" />' },
          FileTree: { template: '<div data-testid="file-tree" />' },
          CodeViewer: { template: '<div data-testid="code-viewer" />' },
          PanelResizer: panelResizerStub,
          CollapsiblePanel: collapsiblePanelStub,
        },
      },
    });

    expect(wrapper.find('[data-testid="project-panel"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="file-tree"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="code-viewer"]').exists()).toBe(true);
  });

  it("clamps panel widths within min/max bounds", () => {
    const wrapper = mount(App, {
      global: {
        stubs: {
          ProjectPanel: { template: "<div />" },
          FileTree: { template: "<div />" },
          CodeViewer: { template: "<div />" },
          PanelResizer: {
            template: '<div @resize="$emit(\'resize\', 9999)" />',
            emits: ["resize"],
          },
          CollapsiblePanel: collapsiblePanelStub,
        },
      },
    });

    const vm = wrapper.vm as any;
    // The file panel's width is applied through its style.
    const filesWidth = parseInt(vm.filesPanelStyle.width, 10);
    expect(vm.leftWidth).toBeLessThanOrEqual(500);
    expect(filesWidth).toBeLessThanOrEqual(500);
    expect(vm.leftWidth).toBeGreaterThanOrEqual(160);
    expect(filesWidth).toBeGreaterThanOrEqual(160);
  });

  it("hides resizers when panels are collapsed", async () => {
    const wrapper = mount(App, {
      global: {
        stubs: {
          ProjectPanel: { template: "<div />" },
          FileTree: { template: "<div />" },
          CodeViewer: { template: "<div />" },
          PanelResizer: panelResizerStub,
          CollapsiblePanel: collapsiblePanelStub,
        },
      },
    });

    expect(wrapper.findAll('[data-testid="panel-resizer"]')).toHaveLength(2);

    const vm = wrapper.vm as any;
    vm.leftCollapsed = true;
    await wrapper.vm.$nextTick();
    expect(wrapper.findAll('[data-testid="panel-resizer"]')).toHaveLength(1);

    vm.middleCollapsed = true;
    await wrapper.vm.$nextTick();
    expect(wrapper.findAll('[data-testid="panel-resizer"]')).toHaveLength(0);
  });

  it("starts in horizontal layout by default", () => {
    const wrapper = mount(App, {
      global: {
        stubs: {
          ProjectPanel: { template: "<div />" },
          FileTree: { template: "<div />" },
          CodeViewer: { template: "<div />" },
          PanelResizer: panelResizerStub,
          CollapsiblePanel: collapsiblePanelStub,
        },
      },
    });

    const vm = wrapper.vm as any;
    expect(vm.leftLayout).toBe('horizontal');
  });

  it("toggles between horizontal and vertical layout", async () => {
    const wrapper = mount(App, {
      global: {
        stubs: {
          ProjectPanel: { template: "<div />" },
          FileTree: { template: "<div />" },
          CodeViewer: { template: "<div />" },
          PanelResizer: panelResizerStub,
          CollapsiblePanel: collapsiblePanelStub,
        },
      },
    });

    const vm = wrapper.vm as any;
    expect(vm.leftLayout).toBe('horizontal');

    vm.toggleLeftLayout();
    await wrapper.vm.$nextTick();
    expect(vm.leftLayout).toBe('vertical');

    vm.toggleLeftLayout();
    await wrapper.vm.$nextTick();
    expect(vm.leftLayout).toBe('horizontal');
  });

  it("keeps the same panels when switching layout instead of rebuilding them", async () => {
    let mounts = 0;
    const wrapper = mount(App, {
      global: {
        stubs: {
          ProjectPanel: { template: "<div />", mounted: () => (mounts += 1) },
          FileTree: { template: "<div />", mounted: () => (mounts += 1) },
          CodeViewer: { template: "<div />" },
          PanelResizer: panelResizerStub,
          CollapsiblePanel: collapsiblePanelStub,
        },
      },
    });

    const vm = wrapper.vm as any;
    expect(mounts).toBe(2);
    vm.toggleLeftLayout();
    await wrapper.vm.$nextTick();
    vm.toggleLeftLayout();
    await wrapper.vm.$nextTick();
    expect(mounts).toBe(2);
  });
});