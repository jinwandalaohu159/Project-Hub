export interface Project {
  id: string;
  name: string;
  path: string;
  is_git_repo: boolean;
  added_at: string;
  /** The folder no longer exists at `path` (renamed, moved or on another drive). */
  missing: boolean;
}

export interface RelocatedProject {
  id: string;
  name: string;
  old_path: string;
  new_path: string;
}

export interface Branch {
  name: string;
  is_current: boolean;
  last_commit_time: string | null;
  last_commit_message: string | null;
}

export type FileStatus = "Modified" | "Added" | "Deleted" | "Renamed" | "Untracked";

export interface FileDiff {
  path: string;
  status: FileStatus;
  old_path?: string | null;
}

export interface StatusSummary {
  files: FileDiff[];
  total: number;
  truncated: boolean;
}

export interface CommitEntry {
  hash: string;
  message: string;
  body: string;
  author: string;
  time: string | null;
  is_current: boolean;
}

export interface DraftInfo {
  hash: string;
  base: string;
  branch: string;
  time: string | null;
  files: number;
}

export interface VersioningOutcome {
  initialized: boolean;
  initial_version: boolean;
}

export interface DeleteRecordOutcome {
  /** The current record was deleted: files stayed, its changes are now unsaved changes. */
  kept_as_unsaved: boolean;
}

export interface RestoreOutcome {
  draft_saved: boolean;
}

export interface RepoState {
  branch: string;
  head: string;
  draft: string | null;
}

export interface SyncState {
  remote_url: string | null;
  branch: string;
  has_upstream: boolean;
  ahead: number;
  behind: number;
}

/** Result of downloading from the cloud. */
export interface PullOutcome {
  /** Save records that came down; 0 when this computer was already up to date. */
  received: number;
}

export interface GithubAccount {
  login: string;
  id: number;
  name: string | null;
  avatar_url: string | null;
  email: string | null;
}

/** A browser sign-in in progress: the code the user enters at `verification_uri`. */
export interface DeviceCode {
  user_code: string;
  verification_uri: string;
  expires_in: number;
}

export interface RepoInfo {
  full_name: string;
  clone_url: string;
  html_url: string;
  private: boolean;
}

export interface SettingsView {
  author_name: string;
  author_email: string;
  effective_name: string;
  effective_email: string;
  github: GithubAccount | null;
}

export interface McpStatus {
  enabled: boolean;
  port: number;
  running: boolean;
  error: string | null;
  url: string;
  exe_path: string;
}

/** Something that went wrong with one project; `message` is an error code (see utils/errors). */
export interface ProjectIssue {
  project: string;
  message: string;
}

export interface ExportReport {
  exported: number;
  warnings: ProjectIssue[];
}

export interface ImportPreviewItem {
  id: string;
  name: string;
  original_path: string;
  suggested_path: string;
  has_history: boolean;
  has_draft: boolean;
  /** Files outside the version history (local settings such as .env) carried in the package. */
  local_files: number;
  registered: boolean;
}

export interface ImportPreview {
  exported_at: string;
  author_name: string;
  author_email: string;
  projects: ImportPreviewItem[];
}

export type ImportTargetState = "missing" | "empty" | "files" | "repo";

export interface ImportReport {
  imported: string[];
  errors: ProjectIssue[];
}
