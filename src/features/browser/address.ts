const LOCAL = /^(localhost|127\.0\.0\.1|\[::1\])(:\d+)?([/?#]|$)/i;

export function webAddress(typed: string): string | null {
  const text = typed.trim();
  if (!text || /\s/.test(text)) return null;
  const scheme = /^([a-z][a-z\d+.-]*):(?!\d)/i.exec(text)?.[1];
  if (scheme && !/^https?$/i.test(scheme)) return null;
  const withScheme = /^https?:\/\//i.test(text) ? text : LOCAL.test(text) ? `http://${text}` : text.includes(".") ? `https://${text}` : null;
  if (!withScheme) return null;
  try {
    return new URL(withScheme).href;
  } catch {
    return null;
  }
}

export function pageName(url: string, title: string): string {
  if (title.trim()) return title.trim();
  try {
    return new URL(url).host || url;
  } catch {
    return url;
  }
}
