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
