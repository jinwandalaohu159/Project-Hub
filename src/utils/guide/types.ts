/** The user guide is data: each language lists the same sections, made of these blocks. */

/** Every chapter, in reading order; both languages must list exactly these. */
export const GUIDE_SECTION_IDS = [
  "welcome",
  "quickStart",
  "layout",
  "saving",
  "versions",
  "files",
  "hiding",
  "cloud",
  "agents",
  "transfer",
  "settings",
  "shortcuts",
  "faq",
] as const;

export type GuideSectionId = (typeof GUIDE_SECTION_IDS)[number];

/** Buttons inside the guide that take the reader straight to the thing being explained. */
export type GuideAction = "addProject" | "openSettings";

/**
 * Text in blocks may use **bold** for names of buttons and panels and `code` for things to type.
 * Anything else is shown as written.
 */
export type GuideBlock =
  | { type: "paragraph"; text: string }
  /** A heading inside a chapter. */
  | { type: "heading"; text: string }
  /** Numbered steps to follow in order. */
  | { type: "steps"; items: string[] }
  | { type: "list"; items: string[] }
  /** Words and what they mean, side by side. */
  | { type: "terms"; items: { term: string; text: string }[] }
  | { type: "tip"; text: string }
  | { type: "warning"; text: string }
  | { type: "keys"; items: { keys: string[]; text: string }[] }
  /** A question and its answer (the FAQ). */
  | { type: "qa"; question: string; answer: string }
  | { type: "action"; action: GuideAction; label: string };

export interface GuideSection {
  id: GuideSectionId;
  title: string;
  /** One line under the title, also shown in the table of contents' tooltip. */
  summary: string;
  blocks: GuideBlock[];
}

export interface Guide {
  title: string;
  subtitle: string;
  sections: GuideSection[];
}
