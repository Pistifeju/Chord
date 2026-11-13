// Tests for the Files tree rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import type { FileEntry } from "../../ipc/bindings.ts";
import { emptyTree, treeKeyAction, visibleRows, withTreeMoved } from "./tree.ts";

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

test("arrow keys move between rows, open and close folders, and jump to the parent", () => {
  const closed = visibleRows(tree);
  const open = visibleRows({ ...tree, open: ["src"] });
  assert.deepEqual(treeKeyAction(closed, "src", "ArrowDown"), { focus: "a.md" });
  assert.deepEqual(treeKeyAction(closed, "src", "ArrowUp"), null);
  assert.deepEqual(treeKeyAction(closed, "src", "ArrowRight"), { toggle: "src" });
  assert.deepEqual(treeKeyAction(open, "src", "ArrowRight"), null);
  assert.deepEqual(treeKeyAction(open, "src", "ArrowLeft"), { toggle: "src" });
  assert.deepEqual(treeKeyAction(open, "src/util.ts", "ArrowLeft"), { focus: "src" });
  assert.deepEqual(treeKeyAction(open, "a.md", "ArrowLeft"), null);
});

test("a rename moves the open folders and the selection inside it, and keeps the rest", () => {
  const tree = { ...emptyTree, open: ["src", "src/app", "docs"], selected: "src/app/App.tsx" };
  const moved = withTreeMoved(tree, "src", "lib");
  assert.deepEqual(moved.open, ["lib", "lib/app", "docs"]);
  assert.equal(moved.selected, "lib/app/App.tsx");
  assert.equal(withTreeMoved({ ...tree, selected: "srcx.ts" }, "src", "lib").selected, "srcx.ts");
});
