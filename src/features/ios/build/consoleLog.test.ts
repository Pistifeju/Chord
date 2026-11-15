// Tests for the console's line rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { appendToConsole, emptyConsole } from "./consoleLog.ts";

test("whole lines join the console, each with its tone", () => {
  const first = appendToConsole(emptyConsole, "one\ntwo\n", "output");
  const second = appendToConsole(first, "boom\n", "error");
  assert.deepEqual(second, [
    { text: "one", tone: "output" },
    { text: "two", tone: "output" },
    { text: "boom", tone: "error" },
  ]);
});
