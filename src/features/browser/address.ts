// Rules for a browser tab's address: what the address field's text opens, and the name the tab shows.

const LOCAL = /^(localhost|127\.0\.0\.1|\[::1\])(:\d+)?([/?#]|$)/i;

// The page typed text opens: a web address as is, a bare host over https, a local one over http; anything else opens nothing.
export function webAddress(typed: string): string | null {
  const text = typed.trim();
  if (!text || /\s/.test(text)) return null;
  // A colon before a digit is a port, not a scheme.
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

// The page's title, else its host, else the address itself.
export function pageName(url: string, title: string): string {
  if (title.trim()) return title.trim();
  try {
    return new URL(url).host || url;
  } catch {
    return url;
  }
}
