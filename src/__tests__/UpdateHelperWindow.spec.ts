import { mount, flushPromises } from "@vue/test-utils";
import { describe, it, expect, vi, beforeEach } from "vitest";

const listeners: Array<(event: { payload: unknown }) => void> = [];
let initial: unknown = { phase: "downloading", downloaded: 0, total: 0 };

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => ({ version: "9.9.9", progress: initial })),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_name: string, handler: (event: { payload: unknown }) => void) => {
    listeners.push(handler);
    return () => undefined;
  }),
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ show: vi.fn(async () => undefined), setFocus: vi.fn(async () => undefined) }),
}));

import UpdateHelperWindow from "../components/update/UpdateHelperWindow.vue";
import { useI18n } from "../composables/useI18n";

function emit(payload: unknown) {
  for (const listener of listeners) listener({ payload });
}

describe("the update helper window", () => {
  beforeEach(() => {
    listeners.length = 0;
    useI18n().setLocale("en");
  });

  it("shows real download progress, then restarting", async () => {
    initial = { phase: "downloading", downloaded: 0, total: 0 };
    const wrapper = mount(UpdateHelperWindow);
    await flushPromises();
    expect(wrapper.text()).toContain("New version 9.9.9");

    emit({ phase: "downloading", downloaded: 5 * 1024 * 1024, total: 10 * 1024 * 1024 });
    await flushPromises();
    expect(wrapper.text()).toContain("Downloading");
    expect(wrapper.text()).toContain("5.0 MB / 10.0 MB");

    emit({ phase: "restarting" });
    await flushPromises();
    expect(wrapper.text()).toContain("Restarting");
    wrapper.unmount();
  });

  it("says when the update failed and why", async () => {
    initial = { phase: "failed", error: "UPDATE_INSTALL_FAILED:exit code 3" };
    const wrapper = mount(UpdateHelperWindow);
    await flushPromises();
    expect(wrapper.text()).toContain("Update failed, opening the previous version");
    expect(wrapper.text()).toContain("exit code 3");
    wrapper.unmount();
  });
});
