import type { TranslationKey } from "../composables/useI18n";

type Translate = (key: TranslationKey) => string;

/**
 * What the settings page explains about each MCP tool, in the user's language. The tools
 * themselves (names, parameters, read-only or not) come from the backend; a tool without an
 * entry here still shows, with the backend's English text.
 */
export interface McpToolDoc {
  summary: TranslationKey;
  returns: TranslationKey;
  params?: Record<string, TranslationKey>;
  /** The arguments of an example call; `project` is a real project folder when there is one. */
  example: (project: string, t: Translate) => Record<string, unknown>;
  /** For tools that change the project and so are never tried here. */
  sampleResult?: (t: Translate) => string;
}

const PROJECT: Record<string, TranslationKey> = { project: "mcpParamProject" };

export const MCP_TOOL_DOCS: Record<string, McpToolDoc> = {
  list_projects: {
    summary: "mcpToolListProjects",
    returns: "mcpReturnsListProjects",
    example: () => ({}),
  },
  get_status: {
    summary: "mcpToolGetStatus",
    returns: "mcpReturnsGetStatus",
    params: PROJECT,
    example: (project) => ({ project }),
  },
  get_diff: {
    summary: "mcpToolGetDiff",
    returns: "mcpReturnsGetDiff",
    params: { ...PROJECT, max_bytes: "mcpParamMaxBytes" },
    example: (project) => ({ project, max_bytes: 20000 }),
  },
  save_version: {
    summary: "mcpToolSaveVersion",
    returns: "mcpReturnsSaveVersion",
    params: { ...PROJECT, title: "mcpParamTitle", description: "mcpParamDescription" },
    example: (project, t) => ({ project, title: t("mcpExampleTitle"), description: t("mcpExampleDescription") }),
    sampleResult: (t) => `Saved "${t("mcpExampleTitle")}" to version main (3f2a9c1).`,
  },
  get_history: {
    summary: "mcpToolGetHistory",
    returns: "mcpReturnsGetHistory",
    params: { ...PROJECT, limit: "mcpParamHistoryLimit" },
    example: (project) => ({ project, limit: 5 }),
  },
  get_ignore_rules: {
    summary: "mcpToolGetIgnoreRules",
    returns: "mcpReturnsGetIgnoreRules",
    params: { ...PROJECT, limit: "mcpParamHiddenLimit" },
    example: (project) => ({ project, limit: 50 }),
  },
  update_ignore_rules: {
    summary: "mcpToolUpdateIgnoreRules",
    returns: "mcpReturnsUpdateIgnoreRules",
    params: { ...PROJECT, add: "mcpParamAdd", remove: "mcpParamRemove" },
    example: (project) => ({ project, add: ["dist/", "*.log"] }),
    sampleResult: () =>
      JSON.stringify(
        {
          added: ["dist/", "*.log"],
          removed: [],
          unchanged: [],
          stopped_saving: ["dist/app.js"],
          next: "Call save_version to record the new rules.",
        },
        null,
        2,
      ),
  },
};

/** Where an example points when there is no project yet. */
export const EXAMPLE_PROJECT = "D:/Projects/my-app";
