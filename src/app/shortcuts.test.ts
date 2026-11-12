// Tests for recording a shortcut and for the saved changes to the default keys.
import { test } from "node:test";
import assert from "node:assert/strict";
import { changedShortcut, loadedChanges, recordedShortcut } from "./shortcuts.ts";

const defaults = { a: { key: "k", cmd: true }, b: { key: "j", cmd: true }, c: undefined };
const press = { key: "K", metaKey: true, ctrlKey: false, altKey: false, shiftKey: true };

test("a recorded press needs cmd, ctrl or alt and is not a bare modifier", () => {
  assert.deepEqual(recordedShortcut(press), { key: "k", cmd: true, ctrl: false, alt: false, shift: true });
  assert.equal(recordedShortcut({ ...press, metaKey: false }), undefined);
  assert.equal(recordedShortcut({ ...press, key: "Meta" }), undefined);
});

test("taking a key from another command names it and leaves it without a key", () => {
  const result = changedShortcut(defaults, {}, "c", { key: "k", cmd: true });
  assert.equal(result.lost, "a");
  assert.deepEqual(result.changes, { c: { key: "k", cmd: true }, a: null });
});

test("giving a command its default key back removes the change", () => {
  const result = changedShortcut(defaults, { b: { key: "x", cmd: true } }, "b", { key: "j", cmd: true });
  assert.deepEqual(result.changes, {});
  assert.equal(result.lost, undefined);
});

test("saved changes for missing commands are dropped, and a clash loads the defaults", () => {
  assert.deepEqual(loadedChanges(defaults, { gone: { key: "x", cmd: true }, c: { key: "x", cmd: true } }), { c: { key: "x", cmd: true } });
  assert.deepEqual(loadedChanges(defaults, { c: { key: "k", cmd: true } }), {});
});
