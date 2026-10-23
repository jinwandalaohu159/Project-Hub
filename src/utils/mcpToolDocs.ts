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
  upload_to_cloud: {
    summary: "mcpToolUploadToCloud",
    returns: "mcpReturnsUploadToCloud",
    params: {
      ...PROJECT,
      new_repository: "mcpParamNewRepository",
      private: "mcpParamPrivate",
      description: "mcpParamRepoDescription",
      overwrite_cloud: "mcpParamOverwriteCloud",
    },
    example: (project) => ({ project }),
    sampleResult: () => "Uploaded version main.",
  },
  download_from_cloud: {
    summary: "mcpToolDownloadFromCloud",
    returns: "mcpReturnsDownloadFromCloud",
    params: { ...PROJECT, keep: "mcpParamKeep" },
    example: (project) => ({ project }),
    sampleResult: () => "Downloaded 2 save record(s) of version main.",
  },
  get_release_info: {
    summary: "mcpToolGetReleaseInfo",
    returns: "mcpReturnsGetReleaseInfo",
    params: PROJECT,
    example: (project) => ({ project }),
  },
  publish_release: {
    summary: "mcpToolPublishRelease",
    returns: "mcpReturnsPublishRelease",
    params: {
      ...PROJECT,
      version: "mcpParamVersion",
      title: "mcpParamReleaseTitle",
      notes: "mcpParamNotes",
      files: "mcpParamFiles",
      cloud_build: "mcpParamCloudBuild",
      prerelease: "mcpParamPrerelease",
    },
    example: (project) => ({ project, version: "1.2.0", files: [`${project}/dist/my-app-1.2.0-setup.exe`] }),
    sampleResult: () =>
      JSON.stringify(
        {
          published: "v1.2.0",
          url: "https://github.com/me/my-app/releases/tag/v1.2.0",
          uploaded: [{ name: "my-app-1.2.0-setup.exe", size: 8421376 }],
          all_files: ["my-app-1.2.0-setup.exe"],
        },
        null,
        2,
      ),
  },
  get_release_status: {
    summary: "mcpToolGetReleaseStatus",
    returns: "mcpReturnsGetReleaseStatus",
    params: { ...PROJECT, version: "mcpParamStatusVersion" },
    example: (project) => ({ project, version: "1.2.0" }),
  },
};

/** Where an example points when there is no project yet. */
export const EXAMPLE_PROJECT = "D:/Projects/my-app";
