// Tests for the attachment rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { fileName, imageMime } from "./attachments.ts";

test("an image file is told by its extension, whatever its case", () => {
  assert.equal(imageMime("/a/shot.PNG"), "image/png");
  assert.equal(imageMime("/a/photo.jpeg"), "image/jpeg");
  assert.equal(imageMime("/a/notes.txt"), null);
  assert.equal(imageMime("/a/folder"), null);
});

test("a file name is the last part of its path", () => {
  assert.equal(fileName("/a/b/shot.png"), "shot.png");
});
