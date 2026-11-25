import assert from "node:assert/strict";
import { test } from "node:test";
import { matchingOptions, numberedIndex, typedIndex } from "./dialog.ts";

test("a query keeps the options that contain it, in order and ignoring case", () => {
  assert.deepEqual(matchingOptions(["Claude Opus", "GPT-5", "claude Haiku"], " CLAUDE "), [0, 2]);
  assert.deepEqual(matchingOptions(["a", "b"], ""), [0, 1]);
});

test("typing jumps to an option whose word starts with it, else one that contains it", () => {
  assert.equal(typedIndex(["Claude Opus", "GPT-5", "Claude Sonnet"], "son"), 2);
  assert.equal(typedIndex(["Claude Opus", "GPT-5"], "5"), 1);
  assert.equal(typedIndex(["Claude Opus", "GPT-5"], "pt"), 1);
  assert.equal(typedIndex(["Claude Opus"], "x"), -1);
});

test("a new name typed over an old one finds the new one", () => {
  assert.equal(typedIndex(["Claude Opus", "Fable"], "opusfa"), 1);
  assert.equal(typedIndex(["Claude Opus", "Fable"], "opusfable"), 1);
});

test("a digit picks the option with that number, within the list", () => {
  assert.equal(numberedIndex("1", 2), 0);
  assert.equal(numberedIndex("3", 2), -1);
  assert.equal(numberedIndex("0", 2), -1);
  assert.equal(numberedIndex("a", 2), -1);
});
