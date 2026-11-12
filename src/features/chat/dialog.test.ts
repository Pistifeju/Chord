// Tests for the select dialog rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { matchingOptions, steppedIndex } from "./dialog.ts";

test("a query keeps the options that contain it, in order and ignoring case", () => {
  assert.deepEqual(matchingOptions(["Claude Opus", "GPT-5", "claude Haiku"], " CLAUDE "), ["Claude Opus", "claude Haiku"]);
  assert.deepEqual(matchingOptions(["a", "b"], ""), ["a", "b"]);
});

test("the highlight wraps at both ends", () => {
  assert.equal(steppedIndex(2, 1, 3), 0);
  assert.equal(steppedIndex(0, -1, 3), 2);
  assert.equal(steppedIndex(0, 1, 0), -1);
});
