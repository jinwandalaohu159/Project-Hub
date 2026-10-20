import type { TranslationKey } from "../composables/useI18n";

type Translate = (key: TranslationKey, params?: Record<string, string | number>) => string;

const CODE_KEYS: Record<string, TranslationKey> = {
  GITHUB_NOT_SIGNED_IN: "errorGithubNotSignedIn",
  GCM_NOT_AVAILABLE_WINDOWS: "errorGcmNotAvailableWindows",
  GCM_NOT_AVAILABLE_MACOS: "errorGcmNotAvailableMacos",
  GCM_NOT_AVAILABLE_LINUX: "errorGcmNotAvailableLinux",
  GITHUB_TOKEN_EMPTY: "errorTokenEmpty",
  INVALID_EMAIL: "errorInvalidEmail",
  INVALID_PORT: "errorInvalidPort",
  UPDATE_RATE_LIMITED: "errorUpdateRateLimited",
  RECORD_UNDELETABLE: "errorRecordUndeletable",
  RECORD_NOT_IN_VERSION: "errorRecordNotInVersion",
};

const PATTERN_KEYS: Array<[RegExp, TranslationKey]> = [
  [/Project folder not found/i, "errorProjectFolderMissing"],
  [/index\.lock/i, "errorGitBusy"],
  [/Could not resolve host|Cannot reach GitHub|unable to access/i, "errorNetwork"],
  [/Authentication failed|token invalid or expired|could not read Username/i, "errorAuthFailed"],
  [/name already exists on this account/i, "errorRepoExists"],
  [/Repository not found/i, "errorRepoNotFound"],
  [/timed out/i, "errorTimeout"],
  [/sign-in was cancelled/i, "errorLoginCancelled"],
];

export function rawError(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error ?? "");
}

/** Error codes from the backend, else the git message without the "git … failed:" prefix. */
export function errorText(error: unknown, t: Translate, fallback?: TranslationKey): string {
  const raw = rawError(error).trim();
  const code = raw.split(":")[0];
  if (CODE_KEYS[code]) return t(CODE_KEYS[code]);
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
