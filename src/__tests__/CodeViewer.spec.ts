import { describe, it, expect } from "vitest";
import fs from "fs";
import path from "path";

describe("CodeViewer empty state", () => {
  it("does not render selectFileToView text in template", () => {
    const vuePath = path.resolve(__dirname, "../components/CodeViewer.vue");
    const content = fs.readFileSync(vuePath, "utf-8");

    expect(content).not.toContain("selectFileToView");
    expect(content).not.toContain("font-serif text-[28px]");
  });

  it("editorContainer does not have flex centering classes for empty state", () => {
    const vuePath = path.resolve(__dirname, "../components/CodeViewer.vue");
    const content = fs.readFileSync(vuePath, "utf-8");

    expect(content).not.toContain("'flex items-center justify-center': !selectedFilePath");
  });
});
