import { mount, flushPromises } from "@vue/test-utils";
import { describe, it, expect, vi, beforeEach } from "vitest";

const commits = [
  { hash: "c3".padEnd(40, "0"), message: "three", body: "", author: "me", time: null, is_current: true },
  { hash: "c2".padEnd(40, "0"), message: "two", body: "details", author: "me", time: null, is_current: false },
  { hash: "c1".padEnd(40, "0"), message: "one", body: "", author: "me", time: null, is_current: false },
];

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (command: string) => {
    if (command === "get_log") return commits.map((commit) => ({ ...commit }));
    if (command === "get_branches") return [];
    return null;
  }),
}));

import BranchManager from "../components/BranchManager.vue";
import CommitLogPanel from "../components/CommitLogPanel.vue";
import { useProjectStore } from "../composables/useProjectStore";

describe("version actions panel", () => {
  it("folds away when clicking elsewhere or pressing Escape", async () => {
    const wrapper = mount(BranchManager, { attachTo: document.body });
    await wrapper.findAll("button")[0].trigger("click");
    expect(wrapper.find("input").exists()).toBe(true);

    document.body.dispatchEvent(new Event("pointerdown", { bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.find("input").exists()).toBe(false);

    await wrapper.findAll("button")[0].trigger("click");
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await wrapper.vm.$nextTick();
    expect(wrapper.find("input").exists()).toBe(false);
    wrapper.unmount();
  });

  it("stays open while clicking inside it", async () => {
    const wrapper = mount(BranchManager, { attachTo: document.body });
    await wrapper.findAll("button")[0].trigger("click");
    wrapper.find("input").element.dispatchEvent(new Event("pointerdown", { bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.find("input").exists()).toBe(true);
    wrapper.unmount();
  });
});

describe("save record menu", () => {
  beforeEach(() => {
    const store = useProjectStore();
    store.projects.value = [
      { id: "p1", name: "Demo", path: "D:/Demo", is_git_repo: true, added_at: "", missing: false },
    ];
    store.selectedProjectId.value = "p1";
  });

  it("opens on right-click with edit and delete", async () => {
    const wrapper = mount(CommitLogPanel, { attachTo: document.body });
    await flushPromises();
    const rows = wrapper.findAll(".group.pl-5");
    expect(rows.length).toBe(3);

    await rows[1].trigger("contextmenu", { clientX: 20, clientY: 20 });
    const menu = document.querySelector(".context-menu");
    expect(menu).not.toBeNull();
    const buttons = menu!.querySelectorAll("button");
    expect(buttons.length).toBe(2);
    expect((buttons[1] as HTMLButtonElement).disabled).toBe(false);

    document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await wrapper.vm.$nextTick();
    expect(document.querySelector(".context-menu")).toBeNull();
    wrapper.unmount();
  });
});
