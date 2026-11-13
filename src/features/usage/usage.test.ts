// Tests for the usage rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { openWindows, usageText } from "./usage.ts";

test("a window whose reset has passed is hidden", () => {
  const windows = [
    { label: "5h 42%", resets_at: 2000 },
    { label: "week 10%", resets_at: 1000 },
    { label: "month 1%", resets_at: null },
  ];
  assert.deepEqual(openWindows(windows, 1500).map((window) => window.label), ["5h 42%", "month 1%"]);
});

test("a window reads as its percentage and the time left in two units", () => {
  const now = 1000;
  assert.equal(usageText({ label: "5h 42%", resets_at: now + (2 * 60 + 48) * 60000 }, now), "42% 2h 48m");
  assert.equal(usageText({ label: "week 59%", resets_at: now + (4 * 24 + 6) * 3600000 + 59000 }, now), "59% 4d 6h");
  assert.equal(usageText({ label: "5h 3%", resets_at: now + 12 * 60000 }, now), "3% 12m");
  assert.equal(usageText({ label: "month", resets_at: null }, now), "month");
});
