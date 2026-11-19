// Tests for the composer's send, key, drop and picker rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { composerKey, droppedFiles, pickerClick, sendable } from "./composer.ts";

const rows = [
  { name: "model", description: "", kind: "session" as const, origin: null },
  { name: "mode", description: "", kind: "session" as const, origin: null },
];
const enter = { key: "Enter", shift: false, composing: false };

test("return on a command typed in full sends it, on a partial name completes the picked row", () => {
  assert.deepEqual(composerKey(enter, "/model", rows, 0), { kind: "send" });
  assert.deepEqual(composerKey(enter, "/mo", rows, 0), { kind: "complete", command: rows[0] });
});

test("tab completes and arrows wrap through the rows", () => {
  assert.deepEqual(composerKey({ ...enter, key: "Tab" }, "/model", rows, 1), { kind: "complete", command: rows[1] });
  assert.deepEqual(composerKey({ ...enter, key: "ArrowUp" }, "/mo", rows, 0), { kind: "step", to: 1 });
});

test("shift return and keys during composition do nothing", () => {
  assert.equal(composerKey({ ...enter, shift: true }, "hi", [], 0), null);
  assert.equal(composerKey({ ...enter, composing: true }, "/mo", rows, 0), null);
});

test("a send needs text or an attachment", () => {
  assert.equal(sendable(true, "  ", 0), false);
  assert.equal(sendable(true, "  ", 1), true);
  assert.equal(sendable(false, "hi", 0), false);
});

test("a drop attaches images while the chat takes them and writes other paths, quoted when spaced", () => {
  assert.deepEqual(droppedFiles(["/a/shot.png", "/a/my clip.mov"], true), { attach: ["/a/shot.png"], draft: ['"/a/my clip.mov"'] });
  assert.deepEqual(droppedFiles(["/a/shot.png"], false), { attach: [], draft: ["/a/shot.png"] });
});

test("a picker button closes its own picker and swaps from the other one", () => {
  assert.deepEqual(pickerClick("Model", "Model"), { open: false });
  assert.deepEqual(pickerClick("Thinking", "Model"), { open: true });
  assert.deepEqual(pickerClick(null, "Model"), { open: true });
});
