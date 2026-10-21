import type { TranslationKey } from "../composables/useI18n";

type Translate = (key: TranslationKey, params?: Record<string, string | number>) => string;

const CODE_KEYS: Record<string, TranslationKey> = {
  GITHUB_NOT_SIGNED_IN: "errorGithubNotSignedIn",
  GITHUB_LOGIN_EXPIRED: "errorLoginExpired",
  GITHUB_LOGIN_DENIED: "errorLoginDenied",
  GITHUB_LOGIN_FAILED: "errorLoginFailed",
  GITHUB_UNAUTHORIZED: "errorAuthFailed",
  PROJECT_FOLDER_MISSING: "errorProjectFolderMissing",
  UPDATE_UNTRUSTED: "errorUpdateUntrusted",
  UPDATE_DOWNLOAD_FAILED: "errorUpdateDownloadFailed",
  UPDATE_INCOMPLETE: "errorUpdateIncomplete",
  UPDATE_INSTALL_FAILED: "errorUpdateInstallFailed",
  UPDATE_NEEDS_APPIMAGE: "errorUpdateNeedsAppImage",
  REPO_URL_EMPTY: "errorRepoUrlEmpty",
  REPO_URL_INVALID: "errorRepoUrlInvalid",
  TOKEN_ENCRYPT_FAILED: "errorTokenEncrypt",
  TOKEN_DECRYPT_FAILED: "errorTokenDecrypt",
  AUTOSTART_UNCONFIRMED: "errorAutostartUnconfirmed",
  EXPORT_NOT_PROJECT_HUB: "errorExportNotProjectHub",
  EXPORT_TOO_NEW: "errorExportTooNew",
  EXPORT_DAMAGED: "errorExportDamaged",
  EXPORT_NO_FILES: "errorExportNoFiles",
  VERSION_DETACHED: "errorVersionDetached",
  VERSION_EXISTS: "errorVersionExists",
  REMOTE_NOT_LINKED: "errorRemoteNotLinked",
  TARGET_HAS_HISTORY: "errorTargetHasHistory",
  TITLE_EMPTY: "errorTitleEmpty",
  PATH_NOT_FOUND: "errorPathNotFound",
  NOT_A_FOLDER: "errorNotAFolder",
  PROJECT_EXISTS: "errorProjectExists",
  FOLDER_IN_USE: "errorFolderInUse",
  NAME_EMPTY: "errorNameEmpty",
  NAME_SPACES: "errorNameSpaces",
  NAME_INVALID_CHARS: "errorNameInvalidChars",
  NAME_TAKEN: "errorNameTaken",
  MOVE_INTO_SELF: "errorMoveIntoSelf",
  MOVE_INTO_CHILD: "errorMoveIntoChild",
  EXPORT_FOLDER_MISSING: "errorExportFolderMissing",
  EXPORT_CREATE_FAILED: "errorExportCreateFailed",
  PACKAGE_OPEN_FAILED: "errorPackageOpenFailed",
  PACKAGE_ERROR: "errorPackage",
  IMPORT_NO_TARGET: "errorImportNoTarget",
  LOCAL_FILE_FAILED: "errorLocalFileFailed",
  LOCAL_FILE_OUTSIDE: "errorLocalFileOutside",
  SYNC_DIVERGED: "errorSyncDiverged",
  SYNC_LOCAL_CHANGES: "errorSyncLocalChanges",
  SYNC_NOT_IN_CLOUD: "errorSyncNotInCloud",
  UNSAVED_BEFORE_BLANK_VERSION: "errorUnsavedBeforeBlank",
  NO_DRAFT: "errorNoDraft",
  DRAFT_NO_BASE: "errorDraftNoBase",
  VERSION_NO_RECORDS: "errorVersionNoRecords",
  PROJECT_NOT_FOUND: "errorProjectNotFound",
  APP_DIR_UNKNOWN: "errorAppDirUnknown",
  GITHUB_API_ERROR: "errorGithubApi",
  GIT_NOT_RUNNABLE: "errorGitNotRunnable",
  FILE_ERROR: "errorFile",
  GITHUB_TOKEN_EMPTY: "errorTokenEmpty",
  INVALID_EMAIL: "errorInvalidEmail",
  INVALID_PORT: "errorInvalidPort",
  UPDATE_RATE_LIMITED: "errorUpdateRateLimited",
  RECORD_UNDELETABLE: "errorRecordUndeletable",
  RECORD_NOT_IN_VERSION: "errorRecordNotInVersion",
};

const PATTERN_KEYS: Array<[RegExp, TranslationKey]> = [
  [/index\.lock/i, "errorGitBusy"],
  [/Could not resolve host|Cannot reach GitHub|unable to access/i, "errorNetwork"],
  [/Authentication failed|could not read Username/i, "errorAuthFailed"],
  [/name already exists on this account/i, "errorRepoExists"],
  [/Repository not found/i, "errorRepoNotFound"],
  [/timed out/i, "errorTimeout"],
];

export function rawError(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error ?? "");
}

/**
 * Error codes from the backend ("CODE", or "CODE:detail" with the detail shown in the message),
 * else the git message without the "git … failed:" prefix.
 */
export function errorText(error: unknown, t: Translate, fallback?: TranslationKey): string {
  const raw = rawError(error).trim();
  const code = raw.split(":")[0];
  if (CODE_KEYS[code]) return t(CODE_KEYS[code], { detail: raw.slice(code.length + 1) });
  for (const [pattern, key] of PATTERN_KEYS) {
    if (pattern.test(raw)) return t(key);
  }
  const cleaned = raw
    .replace(/^git .*? failed:\s*/i, "")
    .replace(/^(fatal|error):\s*/i, "")
    .replace(/\s+/g, " ")
    .trim();
  return cleaned || (fallback ? t(fallback) : t("unknownError"));
}

export function isErrorCode(error: unknown, code: string): boolean {
  return rawError(error).startsWith(code);
}
