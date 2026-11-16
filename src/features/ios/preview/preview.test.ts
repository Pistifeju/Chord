// Tests for the Preview view's rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { appliedState, previewLine } from "./preview.ts";

test("a rebuild keeps the shown preview, a new boot hides it", () => {
  const live = appliedState(undefined, { state: "live" });
  const rebuilding = appliedState(live, { state: "building" });
  assert.equal(previewLine(rebuilding), null);
  assert.equal(previewLine(appliedState(rebuilding, { state: "starting" })), "Starting the preview…");
});

test("the limit names the worktrees whose previews run", () => {
  assert.equal(previewLine(appliedState(undefined, { state: "full", worktrees: ["login", "onboarding"] })), "Two previews already run: login, onboarding.");
});
