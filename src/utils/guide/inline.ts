/** A run of guide text: plain, **bold** (a button or panel name) or `code` (something to type). */
export interface InlinePart {
  kind: "text" | "strong" | "code";
  text: string;
}

const MARKUP = /\*\*(.+?)\*\*|`(.+?)`/g;

/** Splits guide text into runs; unmatched markers stay as plain text. */
export function parseInline(text: string): InlinePart[] {
  const parts: InlinePart[] = [];
  let last = 0;
  for (const match of text.matchAll(MARKUP)) {
    const index = match.index ?? 0;
    if (index > last) parts.push({ kind: "text", text: text.slice(last, index) });
    parts.push(match[1] !== undefined ? { kind: "strong", text: match[1] } : { kind: "code", text: match[2] });
    last = index + match[0].length;
  }
  if (last < text.length) parts.push({ kind: "text", text: text.slice(last) });
  return parts;
}
