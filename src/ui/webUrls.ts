// Bare web URLs in text, shared by the chat's Markdown and the terminal's links.

// Quotes and angle brackets end a URL so `"https://a.dev","https://b.dev"` in JSON output gives two links.
const WEB_URL = /https?:\/\/[^\s<>"']+/gi;
const WEB_URL_AT = new RegExp(WEB_URL.source, "iy");

/** The web URLs in `text`, each with its offset. */
export function webUrlsIn(text: string): { url: string; start: number }[] {
  return [...text.matchAll(WEB_URL)].map((m) => ({ url: trimmedUrl(m[0]), start: m.index }));
}

/** The web URL that starts at `index` in `text`, if one does. */
export function webUrlAt(text: string, index: number): string | null {
  WEB_URL_AT.lastIndex = index;
  const match = WEB_URL_AT.exec(text);
  return match && trimmedUrl(match[0]);
}

// Sentence punctuation after a bare URL is not part of it; a closing ")" is only when the URL opened one.
function trimmedUrl(url: string): string {
  let end = url.replace(/[.,;:!?'"*_~]+$/, "");
  while (end.endsWith(")") && end.split("(").length < end.split(")").length) end = end.slice(0, -1).replace(/[.,;:!?'"*_~]+$/, "");
  return end;
}
