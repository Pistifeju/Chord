// Tests for the Simulator view's screen rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { clampToScreen, cornerRadius, fingersFor, fit, fromBottom, gestureFingers, secondFinger, toScreen } from "./screen.ts";

const rect = { x: 10, y: 20, width: 100, height: 200 };

test("the screen fits the view, centered, keeping its shape", () => {
  assert.deepEqual(fit({ x: 0, y: 40, width: 400, height: 200 }, { width: 1000, height: 2000 }), { x: 150, y: 40, width: 100, height: 200 });
  assert.deepEqual(fit({ x: 0, y: 0, width: 100, height: 400 }, { width: 1000, height: 2000 }), { x: 0, y: 100, width: 100, height: 200 });
  assert.deepEqual(fit({ x: 5, y: 6, width: 100, height: 100 }, { width: 0, height: 0 }), { x: 5, y: 6, width: 0, height: 0 });
});

test("an iPhone's screen rounds its corners far more than an iPad's", () => {
  assert.equal(cornerRadius({ x: 0, y: 0, width: 393, height: 852 }), 47);
  assert.equal(cornerRadius({ x: 0, y: 0, width: 820, height: 1180 }), 18);
  assert.equal(cornerRadius({ x: 0, y: 0, width: 1180, height: 820 }), 18);
});

test("a pointer on the screen reads from its top left; outside it reads null or pins to the edge", () => {
  assert.deepEqual(toScreen(60, 120, rect), { x: 0.5, y: 0.5 });
  assert.deepEqual(toScreen(110, 220, rect), { x: 1, y: 1 });
  assert.equal(toScreen(9, 120, rect), null);
  assert.deepEqual(clampToScreen(0, 500, rect), { x: 0, y: 1 });
  assert.equal(fromBottom({ x: 0.5, y: 0.98 }), true);
  assert.equal(fromBottom({ x: 0.5, y: 0.9 }), false);
});

test("⌥ mirrors a second finger, ⌥⇧ keeps its offset", () => {
  assert.deepEqual(fingersFor({ alt: false, shift: true }, { x: 0.2, y: 0.5 }), { kind: "one" });
  assert.equal(secondFinger({ kind: "one" }, { x: 0.2, y: 0.5 }), null);
  assert.deepEqual(secondFinger(fingersFor({ alt: true, shift: false }, { x: 0.25, y: 0.5 }), { x: 0.25, y: 0.75 }), { x: 0.75, y: 0.25 });
  const pan = fingersFor({ alt: true, shift: true }, { x: 0.25, y: 0.5 });
  assert.deepEqual(secondFinger(pan, { x: 0.5, y: 0.25 }), { x: 1, y: 0.25 });
});

test("a trackpad gesture spreads and turns two fingers around its center", () => {
  assert.deepEqual(gestureFingers({ x: 0.5, y: 0.5 }, 1, 0), [{ x: 0.4, y: 0.5 }, { x: 0.6, y: 0.5 }]);
  assert.deepEqual(gestureFingers({ x: 0.5, y: 0.5 }, 2, 0), [{ x: 0.3, y: 0.5 }, { x: 0.7, y: 0.5 }]);
  const [first, second] = gestureFingers({ x: 0.5, y: 0.5 }, 1, 90);
  assert.ok(Math.abs(first.x - 0.5) < 1e-9 && Math.abs(first.y - 0.4) < 1e-9);
  assert.ok(Math.abs(second.x - 0.5) < 1e-9 && Math.abs(second.y - 0.6) < 1e-9);
});
