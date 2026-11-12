// Tests for the finished marker rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { keptMarks, turnedDone } from "./finished.ts";

test("a pane turns done once, and a repeated done report marks nothing", () => {
  assert.deepEqual(turnedDone({ a: "running", b: "done" }, { a: "done", b: "done", c: "done" }), ["a", "c"]);
  assert.deepEqual(turnedDone({ a: "done" }, { a: "running" }), []);
});

test("a mark goes with its pane and with the showing of the pane", () => {
  assert.deepEqual(keptMarks(["a", "b", "c", "a"], { a: "done", b: "done" }, ["b"]), ["a"]);
});
