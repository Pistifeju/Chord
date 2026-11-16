// Tests for the key to HID usage rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { hidUsage } from "./keys.ts";

test("letters, digits and function keys take their HID usages", () => {
  assert.equal(hidUsage("KeyA"), 0x04);
  assert.equal(hidUsage("KeyZ"), 0x1d);
  assert.equal(hidUsage("Digit1"), 0x1e);
  assert.equal(hidUsage("Digit9"), 0x26);
  assert.equal(hidUsage("Digit0"), 0x27);
  assert.equal(hidUsage("F1"), 0x3a);
  assert.equal(hidUsage("F12"), 0x45);
});

test("⌘ and unknown keys take none", () => {
  assert.equal(hidUsage("MetaLeft"), null);
  assert.equal(hidUsage("F13"), null);
  assert.equal(hidUsage("NumpadEnter"), null);
});
