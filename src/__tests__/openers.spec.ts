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
  default: false,
  ...extra,
});

const list: OpenerView[] = [
  opener("terminal", "terminal"),
  opener("hidden-shell", "terminal", { hidden: true }),
  opener("vscode", "editor", { default: true }),
  opener("files", "files"),
  opener("cursor", "editor"),
];

const invoke = vi.fn(async (command: string, _args?: unknown) => (command === "list_openers" ? list : null));
vi.mock("@tauri-apps/api/core", () => ({ invoke: (command: string, args: unknown) => invoke(command, args) }));

import OpenWithButton from "../components/openers/OpenWithButton.vue";
import { useI18n } from "../composables/useI18n";

describe("the open-with menu", () => {
  it("puts the default first, groups the rest by kind and leaves hidden ones out", async () => {
    useI18n().setLocale("en");
    const wrapper = mount(OpenWithButton, { props: { path: "/work/demo" }, attachTo: document.body });
    await flushPromises();
    await wrapper.get("button").trigger("click");
    await flushPromises();

    const items = Array.from(document.querySelectorAll(".context-menu .menu-item")).map((item) => item.textContent?.trim());
    expect(items).toEqual(["vscodeDefault", "terminal", "cursor", "files", "Manage programs…"]);

    (document.querySelectorAll<HTMLButtonElement>(".context-menu .menu-item")[1]).click();
    await flushPromises();
    expect(invoke).toHaveBeenCalledWith("open_folder_with", { id: "terminal", path: "/work/demo" });
    wrapper.unmount();
  });
});
