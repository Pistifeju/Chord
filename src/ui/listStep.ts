export function steppedIndex(at: number, by: 1 | -1, count: number): number {
  return count === 0 ? -1 : (at + by + count) % count;
}

export function steppedPast(at: number, by: 1 | -1, count: number, skipped: (index: number) => boolean): number {
  let next = at;
  for (let step = 0; step < count; step++) {
    next = steppedIndex(next, by, count);
    if (!skipped(next)) return next;
  }
  return at;
}
