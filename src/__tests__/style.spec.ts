import { describe, it, expect } from "vitest";
import fs from "fs";
import path from "path";

describe("style.css font configuration", () => {
  it("uses Inter as primary font with serif fallback", () => {
    const cssPath = path.resolve(__dirname, "../../src/style.css");
    const css = fs.readFileSync(cssPath, "utf-8");

    expect(css).toContain("'Inter'");
    expect(css).toContain("'Cormorant Garamond'");
  });
});

describe("text and icon alignment", () => {
  it("does not use text-[10px] in any component (use text-xs for consistent alignment)", () => {
    const componentsDir = path.resolve(__dirname, "../components");
    const files = fs.readdirSync(componentsDir).filter((f) => f.endsWith(".vue"));

    for (const file of files) {
      const content = fs.readFileSync(path.join(componentsDir, file), "utf-8");
      expect(content, `${file} should not use text-[10px]`).not.toContain("text-[10px]");
    }
  });

  it("style.css sets base font-size and line-height", () => {
    const css = fs.readFileSync(path.resolve(__dirname, "../../src/style.css"), "utf-8");
    expect(css).toContain("font-size:");
    expect(css).toContain("line-height:");
  });
});
