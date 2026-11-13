// Tests for the Open File by Name rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { matchingFiles } from "./openByName.ts";

const paths = ["docs/store.md", "src/app/App.tsx", "src/features/files/store.ts", "src/store/index.ts", "src/ui/restore.ts"];

test("a name that starts with the query comes first, then one that contains it, then a path that does", () => {
  assert.deepEqual(matchingFiles(paths, " STORE "), ["docs/store.md", "src/features/files/store.ts", "src/ui/restore.ts", "src/store/index.ts"]);
});

test("an empty query lists every file", () => {
  assert.deepEqual(matchingFiles(paths, ""), paths);
});

test("a query nothing contains lists none", () => {
  assert.deepEqual(matchingFiles(paths, "zzz"), []);
});
