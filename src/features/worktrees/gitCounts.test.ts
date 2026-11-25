import assert from "node:assert/strict";
import { test } from "node:test";
import { nextCounts } from "./gitCounts.ts";

const counts = (path: string, ahead: number, changed: number) => ({ path, ahead, behind: 0, changed });

test("fresh counts replace the queried cards and keep the others", () => {
  const current = { a: counts("a", 1, 0), b: counts("b", 2, 0) };
  const next = nextCounts(current, ["a", "b", "c"], ["b", "c"], [counts("b", 5, 1), counts("c", 0, 3)]);
  assert.deepEqual(next, { a: counts("a", 1, 0), b: counts("b", 5, 1), c: counts("c", 0, 3) });
});

test("a queried card git no longer reports loses its counts", () => {
  assert.deepEqual(nextCounts({ a: counts("a", 1, 0), b: counts("b", 2, 0) }, ["a", "b"], ["a"], []), { b: counts("b", 2, 0) });
});

test("counts of a worktree deleted while git ran are dropped", () => {
  assert.deepEqual(nextCounts({ a: counts("a", 1, 0) }, ["a"], ["a", "gone"], [counts("a", 1, 0), counts("gone", 4, 4)]), { a: counts("a", 1, 0) });
});

test("the same counts again keep the same object", () => {
  const current = { a: counts("a", 1, 2) };
  assert.equal(nextCounts(current, ["a"], ["a"], [counts("a", 1, 2)]), current);
});
