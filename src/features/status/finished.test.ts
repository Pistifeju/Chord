import assert from "node:assert/strict";
import { test } from "node:test";
import { keptMarks, turnedFinished } from "./finished.ts";

test("a pane turns done, waiting, failed or exited once, and a repeated report marks nothing", () => {
  assert.deepEqual(turnedFinished({ a: "running", b: "done", d: "running", e: "running" }, { a: "done", b: "done", c: "done", d: "error", e: "exited" }), ["a", "c", "d", "e"]);
  assert.deepEqual(turnedFinished({ a: "done", b: "running", c: "waiting" }, { a: "running", b: "waiting", c: "waiting" }), ["b"]);
});

test("a mark goes with its pane and with the showing of the pane", () => {
  assert.deepEqual(keptMarks(["a", "b", "c", "a"], { a: "done", b: "done" }, ["b"]), ["a"]);
});
