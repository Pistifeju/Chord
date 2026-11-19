// The highlighted row of a list after an arrow key or a find step, also one that passes over rows nobody can pick.

/** The highlighted index after a step of `by`, wrapping at both ends; -1 when the list is empty. */
export function steppedIndex(at: number, by: 1 | -1, count: number): number {
  return count === 0 ? -1 : (at + by + count) % count;
}

/** The highlighted index after a step of `by` past the rows `skipped` holds; `at` when every other row is skipped. */
export function steppedPast(at: number, by: 1 | -1, count: number, skipped: (index: number) => boolean): number {
  let next = at;
  for (let step = 0; step < count; step++) {
    next = steppedIndex(next, by, count);
    if (!skipped(next)) return next;
  }
  return at;
}
