// Tests for the Open File by Name rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { matchingFiles, recentFirst } from "./openByName.ts";

const paths = ["docs/store.md", "src/app/App.tsx", "src/features/files/store.ts", "src/store/index.ts", "src/ui/restore.ts"];

test("a file name that starts with the query comes first, then one that holds it, then a folder that does", () => {
  assert.deepEqual(matchingFiles(paths, " STORE "), ["docs/store.md", "src/features/files/store.ts", "src/ui/restore.ts", "src/store/index.ts"]);
});

test("the first letters of a file name's words find it", () => {
  const files = ["src/features/files/openByName.ts", "src/features/files/OpenFileByNamePopup.tsx", "src/ui/PickerSheet.tsx"];
  assert.deepEqual(matchingFiles(files, "ofbn"), ["src/features/files/OpenFileByNamePopup.tsx"]);
});

test("a folder and a file name with a space between find the file", () => {
  assert.deepEqual(matchingFiles(paths, "files store"), ["src/features/files/store.ts"]);
});

test("an empty query lists every file", () => {
  assert.deepEqual(matchingFiles(paths, ""), paths);
});

test("a query nothing holds lists none", () => {
  assert.deepEqual(matchingFiles(paths, "zzz"), []);
});

test("recently opened files come first, latest first, and a gone one drops out", () => {
  assert.deepEqual(recentFirst(["a.ts", "b.ts", "c.ts"], ["c.ts", "gone.ts", "b.ts"]), ["c.ts", "b.ts", "a.ts"]);
});
