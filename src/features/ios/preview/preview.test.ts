// Tests for the Preview view's rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { appliedState, previewLine, rebuilding, shownError } from "./preview.ts";

test("a rebuild keeps the shown preview, a new boot hides it", () => {
  const live = appliedState(undefined, { state: "live" });
  const rebuilding = appliedState(live, { state: "building" });
  assert.equal(previewLine(rebuilding), null);
  assert.equal(previewLine(appliedState(rebuilding, { state: "starting" })), "Starting the preview…");
});

test("the header says Rebuilding only while a build replaces a shown preview", () => {
  const building = appliedState(undefined, { state: "building" });
  assert.equal(rebuilding(building), false);
  const live = appliedState(building, { state: "live" });
  assert.equal(rebuilding(live), false);
  assert.equal(rebuilding(appliedState(live, { state: "waiting" })), true);
});

test("the limit names the worktrees whose previews run", () => {
  assert.equal(previewLine(appliedState(undefined, { state: "full", worktrees: ["login", "onboarding"] })), "Two previews already run: login, onboarding.");
});

test("a gone pick takes the shown preview away and asks for a pick", () => {
  const live = appliedState(undefined, { state: "live" });
  assert.equal(previewLine(appliedState(live, { state: "unpicked" })), "Pick a preview.");
});

test("a compile error keeps the shown preview under its bar, and before the first frame takes the line", () => {
  const issue = { file: "/w/App/LoginView.swift", line: 42, message: "cannot find 'x' in scope" };
  const failed = appliedState(appliedState(undefined, { state: "live" }), { state: "error", issue });
  assert.equal(previewLine(failed), null);
  assert.deepEqual(shownError(failed), issue);
  assert.equal(shownError(appliedState(failed, { state: "live" })), null);
  assert.equal(previewLine(appliedState(undefined, { state: "error", issue })), "LoginView.swift:42: cannot find 'x' in scope");
});
