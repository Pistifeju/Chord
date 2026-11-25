import assert from "node:assert/strict";
import { test } from "node:test";
import { paneChanges, resumedPanes, vanishedWorktrees } from "./agents.ts";
import { firstPanes, withFileOpened, withSplitFocused } from "./sessions.ts";

test("a split starts the new pane and closing a worktree forgets all of its panes", () => {
  const one = firstPanes("s1", "p1");
  const two = withSplitFocused(one, "g2", "p2", "column");
  assert.deepEqual(paneChanges(one, two), { started: ["p2"], gone: [] });
  assert.deepEqual(paneChanges(two, undefined), { started: [], gone: ["p1", "p2"] });
});

test("only running panes resume, and missing worktrees vanish", () => {
  const saved = { "/a": withSplitFocused(firstPanes("g1", "p1"), "g2", "p2", "column"), "/gone": firstPanes("s2", "p3") };
  assert.deepEqual(resumedPanes(saved, ["p2"]), [["/a", "p2"]]);
  assert.deepEqual(vanishedWorktrees(saved, ["/a"]), ["/gone"]);
});

test("a file tab starts no agent", () => {
  const one = firstPanes("g1", "p1");
  const withFile = withFileOpened(one, { kind: "file", id: "f1", path: "src/main.ts" });
  assert.deepEqual(paneChanges(one, withFile), { started: [], gone: [] });
});
