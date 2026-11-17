// The highlighted row of a list after an arrow key or a find step.

/** The highlighted index after a step of `by`, wrapping at both ends; -1 when the list is empty. */
export function steppedIndex(at: number, by: 1 | -1, count: number): number {
  return count === 0 ? -1 : (at + by + count) % count;
}
