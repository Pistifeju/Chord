// Rules for a select dialog: which options match, and how the highlighted one moves.

/** The options that contain the query, ignoring case, in their order. */
export function matchingOptions(options: string[], query: string): string[] {
  const needle = query.trim().toLowerCase();
  return needle ? options.filter((option) => option.toLowerCase().includes(needle)) : options;
}

/** Where the highlight starts: on the current option, else the first. */
export const startIndex = (options: string[], current: string | null): number => Math.max(0, current === null ? 0 : options.indexOf(current));

/** The highlighted index after a step of `by`, wrapping at both ends; -1 when the list is empty. */
export function steppedIndex(at: number, by: 1 | -1, count: number): number {
  return count === 0 ? -1 : (at + by + count) % count;
}

function matchIndex(options: string[], needle: string): number {
  const startsWord = options.findIndex((option) => option.toLowerCase().split(/[\s\-_/.:]+/).some((word) => word.startsWith(needle)));
  return startsWord !== -1 ? startsWord : options.findIndex((option) => option.toLowerCase().includes(needle));
}

/** The option typing jumps to: the longest tail of what was typed that matches wins, so a new name typed over an old one finds the new one. A match is the first option with a word starting with it, else the first containing it; -1 when none does. */
export function typedIndex(options: string[], typed: string): number {
  const text = typed.toLowerCase();
  for (let from = 0; from < text.length; from++) {
    const at = matchIndex(options, text.slice(from));
    if (at !== -1) return at;
  }
  return -1;
}
