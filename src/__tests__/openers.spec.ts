import { mount, flushPromises } from "@vue/test-utils";
import { describe, it, expect, vi } from "vitest";
import type { OpenerView } from "../types";

const opener = (id: string, kind: OpenerView["kind"], extra: Partial<OpenerView> = {}): OpenerView => ({
  id,
  name: id,
  kind,
  program: `/bin/${id}`,
  custom: false,
  hidden: false,
  ...extra,
});

const list: OpenerView[] = [
  opener("terminal", "terminal"),
  opener("hidden-shell", "terminal", { hidden: true }),
  opener("mine", "other", { custom: true }),
  opener("vscode", "editor"),
  opener("files", "files"),
  opener("cursor", "editor"),
];

const invoke = vi.fn(async (command: string, _args?: unknown) => (command === "list_openers" ? list : null));
vi.mock("@tauri-apps/api/core", () => ({ invoke: (command: string, args: unknown) => invoke(command, args) }));

import OpenWithButton from "../components/openers/OpenWithButton.vue";
import { useI18n } from "../composables/useI18n";

describe("the open-with menu", () => {
  it("groups programs by kind under headings and leaves hidden ones out", async () => {
    useI18n().setLocale("en");
    const wrapper = mount(OpenWithButton, { props: { path: "/work/demo" }, attachTo: document.body });
    await flushPromises();
    await wrapper.get("button").trigger("click");
    await flushPromises();

    const entries = Array.from(document.querySelectorAll(".context-menu .opener-group, .context-menu .menu-item")).map((item) =>
      item.textContent?.trim(),
    );
    expect(entries).toEqual([
      "Terminals",
      "Terminal",
      "Editors",
      "vscode",
      "cursor",
      "Folders",
      "Files",
      "Other",
      "mine",
      "Manage programs…",
    ]);

    (document.querySelectorAll<HTMLButtonElement>(".context-menu .menu-item")[0]).click();
    await flushPromises();
    expect(invoke).toHaveBeenCalledWith("open_folder_with", { id: "terminal", path: "/work/demo" });
    wrapper.unmount();
  });

  it("names the system's own programs in the chosen language", async () => {
    useI18n().setLocale("zh");
    const wrapper = mount(OpenWithButton, { props: { path: "/work/demo" }, attachTo: document.body });
    await flushPromises();
    await wrapper.get("button").trigger("click");
    await flushPromises();

    const names = Array.from(document.querySelectorAll(".context-menu .menu-item")).map((item) => item.textContent?.trim());
    expect(names).toContain("终端");
    expect(names).toContain("文件管理器");
    expect(names).toContain("vscode");
    useI18n().setLocale("en");
    wrapper.unmount();
  });
});
