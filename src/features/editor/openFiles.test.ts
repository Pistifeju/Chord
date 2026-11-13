// Tests for the file tab rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { withFileClosed, withFileOpened, withFilesMoved, withSessionsShown } from "./openFiles.ts";

test("opening a file twice keeps one tab and shows it", () => {
  const open = withFileOpened(withFileOpened({ files: [], active: null }, "a.ts"), "b.ts");
  assert.deepEqual(withFileOpened(open, "a.ts"), { files: ["a.ts", "b.ts"], active: "a.ts" });
});

test("closing the shown file shows the tab after it, else the one before, else the sessions", () => {
  const open = { files: ["a", "b", "c"], active: "b" };
  assert.deepEqual(withFileClosed(open, "b"), { files: ["a", "c"], active: "c" });
  assert.deepEqual(withFileClosed({ files: ["a", "b"], active: "b" }, "b"), { files: ["a"], active: "a" });
  assert.deepEqual(withFileClosed({ files: ["a"], active: "a" }, "a"), { files: [], active: null });
});

test("closing another file keeps the shown one; showing the sessions keeps the tabs", () => {
  assert.deepEqual(withFileClosed({ files: ["a", "b"], active: "b" }, "a"), { files: ["b"], active: "b" });
  assert.deepEqual(withSessionsShown({ files: ["a"], active: "a" }), { files: ["a"], active: null });
});

test("a rename moves the tabs of the entry and of what it holds, not of a sibling with the same prefix", () => {
  const open = { files: ["src/a.ts", "src/b/c.ts", "srcx/d.ts"], active: "src/b/c.ts" };
  assert.deepEqual(withFilesMoved(open, "src", "lib"), { files: ["lib/a.ts", "lib/b/c.ts", "srcx/d.ts"], active: "lib/b/c.ts" });
  assert.deepEqual(withFilesMoved({ files: ["a.ts"], active: null }, "a.ts", "b.ts"), { files: ["b.ts"], active: null });
});
