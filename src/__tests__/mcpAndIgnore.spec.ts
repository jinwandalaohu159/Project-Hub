import { mount } from "@vue/test-utils";
import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (command: string) => (command === "try_mcp_tool" ? "[]" : null)),
}));

import { ignoreRuleFor } from "../utils/ignoreRules";
import { MCP_TOOL_DOCS } from "../utils/mcpToolDocs";
import McpToolDetail from "../components/settings/McpToolDetail.vue";
import { useI18n } from "../composables/useI18n";
import { useProjectStore } from "../composables/useProjectStore";
import type { McpTool } from "../types";

describe("ignoreRuleFor", () => {
  it("anchors the rule at the project root and marks folders", () => {
    expect(ignoreRuleFor("D:/Demo/dist", "D:/Demo", true)).toBe("/dist/");
    expect(ignoreRuleFor("D:\\Demo\\src\\app.log", "D:\\Demo\\", false)).toBe("/src/app.log");
  });

  it("escapes characters gitignore would treat as patterns", () => {
    expect(ignoreRuleFor("D:/Demo/[draft]*.md", "D:/Demo", false)).toBe("/\\[draft]\\*.md");
    expect(ignoreRuleFor("D:/Demo/#notes", "D:/Demo", false)).toBe("/\\#notes");
    expect(ignoreRuleFor("D:/Demo/name ", "D:/Demo", false)).toBe("/name\\ ");
  });

  it("refuses paths outside the project", () => {
    expect(ignoreRuleFor("D:/Other/a.txt", "D:/Demo", false)).toBeNull();
    expect(ignoreRuleFor("D:/Demo", "D:/Demo", true)).toBeNull();
  });
});

const readTool: McpTool = {
  name: "get_history",
  description: "List recent save records of the current version.",
  inputSchema: {
    properties: { project: { type: "string" }, limit: { type: "integer" } },
    required: ["project"],
  },
  annotations: { readOnlyHint: true },
};

const writeTool: McpTool = {
  name: "update_ignore_rules",
  description: "Add or remove .gitignore rules.",
  inputSchema: {
    properties: { project: { type: "string" }, add: { type: "array", items: { type: "string" } } },
    required: ["project"],
  },
  annotations: { readOnlyHint: false },
};

describe("MCP tool details", () => {
  beforeEach(() => {
    useI18n().setLocale("en");
    const store = useProjectStore();
    store.projects.value = [{ id: "p1", name: "Demo", path: "D:/Demo", is_git_repo: true, added_at: "", missing: false }];
    store.selectedProjectId.value = "p1";
  });

  it("lists each parameter with its type and whether it is required", () => {
    const wrapper = mount(McpToolDetail, { props: { tool: writeTool } });
    const text = wrapper.text();
    expect(text).toContain("string[]");
    expect(text).toContain("required");
    expect(text).toContain("optional");
    // The example call uses the real project folder.
    expect(text).toContain('"project": "D:/Demo"');
  });

  it("offers to try read-only tools only", async () => {
    const read = mount(McpToolDetail, { props: { tool: readTool } });
    expect(read.text()).toContain("Try it");

    const write = mount(McpToolDetail, { props: { tool: writeTool } });
    expect(write.text()).not.toContain("Try it");
    expect(write.text()).toContain("stopped_saving");
  });

  it("documents every tool the backend offers", () => {
    for (const name of ["list_projects", "get_status", "get_diff", "save_version", "get_history", "get_ignore_rules", "update_ignore_rules"]) {
      expect(MCP_TOOL_DOCS[name], name).toBeDefined();
    }
  });
});
