// Rules for a select dialog: when its list filters, which options match, and how the highlighted one moves.

/** A list of more options than this gets a filter field. */
const FILTER_ABOVE = 8;

export const filters = (options: string[]): boolean => options.length > FILTER_ABOVE;

/** The options that contain the query, ignoring case, in their order. */
export function matchingOptions(options: string[], query: string): string[] {
  const needle = query.trim().toLowerCase();
  return needle ? options.filter((option) => option.toLowerCase().includes(needle)) : options;
}

/** The highlighted index after a step of `by`, wrapping at both ends; -1 when the list is empty. */
export function steppedIndex(at: number, by: 1 | -1, count: number): number {
  return count === 0 ? -1 : (at + by + count) % count;
}
