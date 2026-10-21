import type { Locale } from "../../composables/useI18n";
import type { Guide } from "./types";
import en from "./en";
import zh from "./zh";

export * from "./types";

const GUIDES: Record<Locale, Guide> = { en, zh };

export function guideFor(locale: Locale): Guide {
  return GUIDES[locale];
}
