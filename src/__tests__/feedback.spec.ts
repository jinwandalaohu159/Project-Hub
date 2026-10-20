import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mount } from "@vue/test-utils";
import { useToast } from "../composables/useToast";
import { useConfirm } from "../composables/useConfirm";
import { useI18n } from "../composables/useI18n";
import { errorText, isErrorCode } from "../utils/errors";
import SaveVersionDialog from "../components/SaveVersionDialog.vue";

describe("useToast", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    const { toasts, dismiss } = useToast();
    [...toasts.value].forEach((toast) => dismiss(toast.id));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("dismisses messages automatically", () => {
    const toast = useToast();
    toast.success("saved");
    expect(toast.toasts.value).toHaveLength(1);
    vi.advanceTimersByTime(3000);
    expect(toast.toasts.value).toHaveLength(0);
  });

  it("keeps errors longer than successes", () => {
    const toast = useToast();
    toast.error("boom");
    vi.advanceTimersByTime(3000);
    expect(toast.toasts.value).toHaveLength(1);
    vi.advanceTimersByTime(4000);
    expect(toast.toasts.value).toHaveLength(0);
  });

  it("pauses while hovered and resumes afterwards", () => {
    const toast = useToast();
    const id = toast.info("read me");
    vi.advanceTimersByTime(1000);
    toast.pause(id);
    vi.advanceTimersByTime(10000);
    expect(toast.toasts.value).toHaveLength(1);
    toast.resume(id);
    vi.advanceTimersByTime(3000);
    expect(toast.toasts.value).toHaveLength(0);
  });

  it("merges identical messages instead of stacking", () => {
    const toast = useToast();
    toast.error("same");
    toast.error("same");
    expect(toast.toasts.value).toHaveLength(1);
  });

  it("caps the number of visible messages", () => {
    const toast = useToast();
    for (let i = 0; i < 8; i++) toast.info(`m${i}`);
    expect(toast.toasts.value.length).toBeLessThanOrEqual(4);
    expect(toast.toasts.value[toast.toasts.value.length - 1]?.message).toBe("m7");
  });
});

describe("useConfirm", () => {
  it("resolves with the user's choice", async () => {
    const { confirm, settle, pending } = useConfirm();
    const answer = confirm({ title: "Sure?" });
    expect(pending.value?.title).toBe("Sure?");
    settle(true);
    await expect(answer).resolves.toBe(true);
    expect(pending.value).toBeNull();
  });

  it("cancels a previous dialog when a new one opens", async () => {
    const { confirm, settle } = useConfirm();
    const first = confirm({ title: "one" });
    const second = confirm({ title: "two" });
    await expect(first).resolves.toBe(false);
    settle(true);
    await expect(second).resolves.toBe(true);
  });
});

describe("errorText", () => {
  const { t, setLocale } = useI18n();

  it("strips git command prefixes", () => {
    setLocale("en");
    expect(errorText("git push -u origin main failed: fatal: something broke", t)).toBe("something broke");
  });

  it("maps backend codes to friendly text", () => {
    setLocale("zh");
    expect(errorText("GITHUB_NOT_SIGNED_IN", t)).toBe("请先登录 GitHub");
    expect(isErrorCode("PUSH_REJECTED: git push failed", "PUSH_REJECTED")).toBe(true);
  });

  it("recognises missing project folders", () => {
    setLocale("zh");
    expect(errorText("Project folder not found: D:/old", t)).toContain("找不到项目文件夹");
  });
});

describe("SaveVersionDialog", () => {
  function mountDialog() {
    return mount(SaveVersionDialog, {
      props: {
        open: true,
        saving: false,
        changes: [{ path: "a.txt", status: "Modified" as const }],
        total: 1,
        branch: "main",
      },
      attachTo: document.body,
    });
  }

  it("emits title and description on Ctrl+Enter", async () => {
    const wrapper = mountDialog();
    await wrapper.vm.$nextTick();
    const input = document.body.querySelector<HTMLInputElement>("input.field")!;
    const textarea = document.body.querySelector<HTMLTextAreaElement>("textarea.field")!;
    input.value = "  Fix login  ";
    input.dispatchEvent(new Event("input"));
    textarea.value = "Details here";
    textarea.dispatchEvent(new Event("input"));
    textarea.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", ctrlKey: true, bubbles: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("save")).toEqual([[{ title: "Fix login", description: "Details here" }]]);
    wrapper.unmount();
  });
});
