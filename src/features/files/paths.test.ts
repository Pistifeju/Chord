// Tests for the path rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { fileName } from "./paths.ts";

test("a file name is the last part of its path", () => {
  assert.equal(fileName("/a/b/shot.png"), "shot.png");
});
