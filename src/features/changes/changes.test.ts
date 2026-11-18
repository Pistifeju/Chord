// Tests for the Changes view rules: where the focus lands after rows leave their group.
import assert from "node:assert/strict";
import { test } from "node:test";
import type { Change } from "../../ipc/bindings";
import { landingAfter, nextSelection, type ChangeGroup } from "./changes.ts";

const group: ChangeGroup = {
  title: "Changes",
  side: "unstaged",
  changes: ["a", "b", "c", "d"].map((path) => ({ path, staged: false }) as Change),
};

test("the focus lands on the row after the ones that left, else the one before", () => {
  assert.equal(landingAfter(group, ["unstaged:b", "unstaged:c"]), "unstaged:d");
  assert.equal(landingAfter(group, ["unstaged:c", "unstaged:d"]), "unstaged:b");
  assert.equal(landingAfter(group, ["unstaged:a", "unstaged:b", "unstaged:c", "unstaged:d"]), null);
});

test("a read that drops the selected row selects the next one, else the change in its new group", () => {
  const files = (staged: string[], unstaged: string[]) => [...staged.map((path) => ({ path, staged: true })), ...unstaged.map((path) => ({ path, staged: false }))] as Change[];
  assert.equal(nextSelection(files([], ["a", "b"]), files(["a"], ["b"]), "unstaged:a"), "unstaged:b");
  assert.equal(nextSelection(files([], ["a"]), files(["a"], []), "unstaged:a"), "staged:a");
  assert.equal(nextSelection(files([], ["a"]), files([], []), "unstaged:a"), null);
  assert.equal(nextSelection(files([], ["a", "b"]), files(["b"], ["a"]), "unstaged:a"), null);
});
