// Tests for the usage rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { openWindows } from "./usage.ts";

test("a window whose reset has passed is hidden", () => {
  const windows = [
    { label: "5h 42%", resets_at: 2000 },
    { label: "week 10%", resets_at: 1000 },
    { label: "month 1%", resets_at: null },
  ];
  assert.deepEqual(openWindows(windows, 1500).map((window) => window.label), ["5h 42%", "month 1%"]);
});
