import assert from "node:assert/strict";
import { test } from "node:test";
import { rewrite } from "./folds.ts";

test("a block after the agent's change moves by the inserted text", () => {
  const change = rewrite("a\n{\n  b\n}\n", "a\nnew\n{\n  b\n}\n");
  assert.equal(change.touchesRange(3, 8), false);
  assert.equal(change.mapPos(3), 7);
  assert.equal(change.mapPos(8), 12);
});

test("a block the agent's change reaches into is touched", () => {
  const change = rewrite("a\n{\n  b\n}\n", "a\n{\n  c\n}\n");
  assert.notEqual(change.touchesRange(3, 8), false);
});

test("the same text is no change", () => {
  assert.equal(rewrite("abc", "abc").empty, true);
});
