// Tests for the rules of which panes' agents start and stop.
import assert from "node:assert/strict";
import { test } from "node:test";
import { paneChanges, resumedPanes, vanishedWorktrees } from "./agents.ts";
import { firstPanes, withFileSession, withSplitFocused } from "./sessions.ts";

test("a split starts the new pane and closing a worktree forgets all of its panes", () => {
  const one = firstPanes("s1", "p1");
  const two = withSplitFocused(one, "p2", "column");
  assert.deepEqual(paneChanges(one, two), { started: ["p2"], gone: [] });
  assert.deepEqual(paneChanges(two, undefined), { started: [], gone: ["p1", "p2"] });
});

test("only running panes of existing worktrees resume, and missing worktrees vanish", () => {
  const saved = { "/a": withSplitFocused(firstPanes("s1", "p1"), "p2", "column"), "/gone": firstPanes("s2", "p3") };
  assert.deepEqual(resumedPanes(saved, ["p2", "p3"], ["/a"]), [["/a", "p2"]]);
  assert.deepEqual(vanishedWorktrees(saved, ["/a"]), ["/gone"]);
});

test("a file pane starts no agent; its new session's agent starts", () => {
  const one = firstPanes("s1", "p1");
  const withFile = withFileSession(one, "s2", { kind: "file", id: "f1", path: "src/main.ts" }, "p2", "row");
  assert.deepEqual(paneChanges(one, withFile), { started: ["p2"], gone: [] });
});
