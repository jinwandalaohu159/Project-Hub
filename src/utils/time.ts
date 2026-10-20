import type { TranslationKey } from "../composables/useI18n";

type Translate = (key: TranslationKey, params?: Record<string, string | number>) => string;

/** "just now", "5m ago" … up to 30 days, then the local date. */
export function formatRelativeTime(iso: string | null, t: Translate, now: number = Date.now()): string {
  if (!iso) return "";
  const d = new Date(iso);
  const diffMin = Math.floor((now - d.getTime()) / 60000);
  if (diffMin < 1) return t("justNow");
  if (diffMin < 60) return t("minutesAgo", { n: diffMin });
  const diffH = Math.floor(diffMin / 60);
  if (diffH < 24) return t("hoursAgo", { n: diffH });
  const diffD = Math.floor(diffH / 24);
  if (diffD < 30) return t("daysAgo", { n: diffD });
  return d.toLocaleDateString();
}
