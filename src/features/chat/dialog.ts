export const TYPING_PAUSE_MS = 1000;

export const filtersOptions = (count: number): boolean => count > 4;

export function matchingOptions(options: string[], query: string): number[] {
  const needle = query.trim().toLowerCase();
  return options.flatMap((option, at) => (option.toLowerCase().includes(needle) ? [at] : []));
}

export function numberedIndex(key: string, count: number): number {
  const at = Number(key) - 1;
  return /^[1-9]$/.test(key) && at < count ? at : -1;
}

export const startIndex = (options: string[], current: string | null): number => Math.max(0, current === null ? 0 : options.indexOf(current));

function matchIndex(options: string[], needle: string): number {
  const startsWord = options.findIndex((option) => option.toLowerCase().split(/[\s\-_/.:]+/).some((word) => word.startsWith(needle)));
  return startsWord !== -1 ? startsWord : options.findIndex((option) => option.toLowerCase().includes(needle));
}

export function typedIndex(options: string[], typed: string): number {
  const text = typed.toLowerCase();
  for (let from = 0; from < text.length; from++) {
    const at = matchIndex(options, text.slice(from));
    if (at !== -1) return at;
  }
  return -1;
}
