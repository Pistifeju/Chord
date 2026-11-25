const WEB_URL = /https?:\/\/[^\s<>"']+/gi;
const WEB_URL_AT = new RegExp(WEB_URL.source, "iy");

export function webUrlsIn(text: string): { url: string; start: number }[] {
  return [...text.matchAll(WEB_URL)].map((m) => ({ url: trimmedUrl(m[0]), start: m.index }));
}

export function webUrlAt(text: string, index: number): string | null {
  WEB_URL_AT.lastIndex = index;
  const match = WEB_URL_AT.exec(text);
  return match && trimmedUrl(match[0]);
}

function trimmedUrl(url: string): string {
  let end = url.replace(/[.,;:!?'"*_~]+$/, "");
  while (end.endsWith(")") && end.split("(").length < end.split(")").length) end = end.slice(0, -1).replace(/[.,;:!?'"*_~]+$/, "");
  return end;
}
