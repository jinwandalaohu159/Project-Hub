import { describe, it, expect, beforeEach } from "vitest";
import { useI18n } from "../composables/useI18n";

describe("useI18n", () => {
  beforeEach(() => {
    const { setLocale } = useI18n();
    setLocale("zh");
  });

  it("returns Chinese strings by default", () => {
    const { t } = useI18n();
    expect(t("projects")).toBe("项目");
    expect(t("files")).toBe("文件");
  });

  it("switches to English when setLocale is called", () => {
    const { t, setLocale } = useI18n();
    setLocale("en");
    expect(t("projects")).toBe("Projects");
    expect(t("files")).toBe("Files");
  });

  it("switches back to Chinese", () => {
    const { t, setLocale } = useI18n();
    setLocale("en");
    expect(t("projects")).toBe("Projects");
    setLocale("zh");
    expect(t("projects")).toBe("项目");
  });

  it("replaces params in Chinese translation strings", () => {
    const { t } = useI18n();
    expect(t("minutesAgo", { n: 5 })).toBe("5分钟前");
    expect(t("removeProjectConfirm", { name: "my-project" })).toBe(
      '从项目列表中移除"my-project"？文件夹不会被删除'
    );
  });

  it("replaces params in English translation strings", () => {
    const { t, setLocale } = useI18n();
    setLocale("en");
    expect(t("minutesAgo", { n: 5 })).toBe("5m ago");
    expect(t("removeProjectConfirm", { name: "my-project" })).toBe(
      'Remove "my-project" from the project list? The folder will not be deleted.'
    );
  });

  it("uses '版本' for branches key in Chinese", () => {
    const { t } = useI18n();
    expect(t("branches")).toBe("版本");
  });

  it("uses 'Versions' in English", () => {
    const { t, setLocale } = useI18n();
    setLocale("en");
    expect(t("branches")).toBe("Versions");
  });

  it("returns key as fallback for unknown keys", () => {
    const { t } = useI18n();
    expect(t("nonexistentKey" as any)).toBe("nonexistentKey");
  });
});
