// Tests for the Files tree rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import type { FileEntry } from "../../ipc/bindings.ts";
import { emptyTree, visibleRows } from "./tree.ts";

const dir = (path: string): FileEntry => ({ name: path.split("/").pop()!, path, isDir: true });
const file = (path: string): FileEntry => ({ name: path.split("/").pop()!, path, isDir: false });

const tree = {
  ...emptyTree,
  children: { "": [dir("src"), file("a.md")], src: [file("src/main.ts"), file("src/util.ts")] },
};

test("a closed folder hides its children", () => {
  assert.deepEqual(visibleRows(tree).map((r) => r.entry.path), ["src", "a.md"]);
});

test("an open folder shows its children one level deeper", () => {
  const rows = visibleRows({ ...tree, open: ["src"] });
  assert.deepEqual(rows.map((r) => [r.entry.path, r.depth, r.open]), [["src", 0, true], ["src/main.ts", 1, false], ["src/util.ts", 1, false], ["a.md", 0, false]]);
});

test("a filter keeps the matches and opens the folders above them", () => {
  const rows = visibleRows({ ...tree, filter: "MAIN" });
  assert.deepEqual(rows.map((r) => r.entry.path), ["src", "src/main.ts"]);
});
