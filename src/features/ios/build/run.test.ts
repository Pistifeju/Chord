// Tests for the build state rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { appliedEvent, appliedSideEvent, markerText, noBuild, pathInWorktree, seenEnd } from "./run.ts";

test("a failed build keeps its errors and the count of the ones it did not list", () => {
  let state = appliedEvent(noBuild, { kind: "phase", phase: "preparing" }, 1000);
  state = appliedEvent(state, { kind: "phase", phase: "building" }, 2000);
  state = appliedEvent(state, { kind: "log", text: "one\ntwo" }, 2500);
  state = appliedEvent(state, { kind: "failed", issues: [{ file: "/w/A.swift", line: 4, message: "bad" }], more: 2 }, 3000);
  assert.equal(state.build?.phase, "failed");
  assert.deepEqual(state.log, { lines: ["one"], partial: "two" });
  assert.equal(state.build?.more, 2);
  assert.deepEqual(markerText(state.build!), { text: "Build failed", tone: "failed" });
});

test("the build's time counts from the start of building, not from preparing or waiting", () => {
  let state = appliedEvent(noBuild, { kind: "phase", phase: "preparing" }, 1000);
  assert.equal(state.build?.startedAt, 1000);
  state = appliedEvent(state, { kind: "phase", phase: "waiting" }, 5000);
  assert.equal(state.build?.startedAt, 1000);
  state = appliedEvent(state, { kind: "phase", phase: "building" }, 9000);
  assert.equal(state.build?.startedAt, 9000);
});

test("the card's marker follows the build from building to its end", () => {
  let state = appliedEvent(noBuild, { kind: "phase", phase: "building" }, 1000);
  assert.deepEqual(markerText(state.build!), { text: "Building", tone: "busy" });
  const stopped = appliedEvent(state, { kind: "stopped" }, 2000).build!;
  assert.deepEqual(markerText(stopped), { text: "Build stopped", tone: "stopped" });
  assert.equal(markerText(seenEnd(stopped)), null);
  state = appliedEvent(state, { kind: "succeeded" }, 2000);
  assert.deepEqual(markerText(state.build!), { text: "Build succeeded", tone: "done" });
  assert.equal(markerText(seenEnd(state.build!)), null);
});

test("a run's card says Build succeeded once the build is done, until seen or the app launches, then Running, and nothing once the app exits", () => {
  let state = appliedEvent(noBuild, { kind: "phase", phase: "installing" }, 1000);
  assert.deepEqual(markerText(state.build!), { text: "Build succeeded", tone: "done" });
  assert.equal(markerText(seenEnd(state.build!)), null);
  state = appliedEvent(state, { kind: "phase", phase: "launching" }, 1500);
  assert.deepEqual(markerText(state.build!), { text: "Build succeeded", tone: "done" });
  state = appliedEvent(state, { kind: "phase", phase: "running" }, 2000);
  assert.deepEqual(markerText(state.build!), { text: "Running", tone: "busy" });
  state = appliedEvent(state, { kind: "exited" }, 3000);
  assert.equal(markerText(state.build!), null);
});

test("a build beside the running app shows Building, then Running again, and keeps the app's phase", () => {
  let state = appliedEvent(noBuild, { kind: "phase", phase: "running" }, 1000);
  state = appliedSideEvent(state, { kind: "phase", phase: "building" }, 2000);
  assert.equal(state.build?.phase, "running");
  assert.deepEqual(markerText(state.build!), { text: "Building", tone: "busy" });
  state = appliedSideEvent(state, { kind: "succeeded" }, 3000);
  assert.deepEqual(markerText(state.build!), { text: "Running", tone: "busy" });
});

test("an error's file is a worktree path only inside the worktree", () => {
  assert.equal(pathInWorktree("/w", "/w/App/A.swift"), "App/A.swift");
  assert.equal(pathInWorktree("/w", "/other/A.swift"), null);
  assert.equal(pathInWorktree("/w", "/wx/A.swift"), null);
});
