// Tests for the list highlight's step.
import assert from "node:assert/strict";
import { test } from "node:test";
import { steppedIndex } from "./listStep.ts";

test("the highlight wraps at both ends", () => {
  assert.equal(steppedIndex(2, 1, 3), 0);
  assert.equal(steppedIndex(0, -1, 3), 2);
  assert.equal(steppedIndex(0, 1, 0), -1);
});
