// Tests for the session tab rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import { activeSession, firstPanes, paneNames, tabName, withFocus, withNewSession, withoutPaneClosed, withSplitFocused } from "./sessions.ts";

test("a new session is named after the highest number and opens", () => {
  const two = withNewSession(firstPanes("s1", "p1"), "s2", "p2");
  const three = withNewSession(two, "s3", "p3");
  assert.deepEqual(three.sessions.map((s) => s.name), ["Session 1", "Session 2", "Session 3"]);
  assert.equal(three.active, "s3");
});

test("a split puts the focus on the new pane", () => {
  const split = withSplitFocused(firstPanes("s1", "p1"), "p2", "column");
  assert.equal(activeSession(split).focused, "p2");
  assert.equal(activeSession(split).layout.kind, "split");
});

test("focusing a pane opens its session", () => {
  const panes = withNewSession(firstPanes("s1", "p1"), "s2", "p2");
  assert.equal(withFocus(panes, "p1").active, "s1");
});

test("closing the last pane of a session closes the tab and opens the one beside it", () => {
  const panes = withNewSession(firstPanes("s1", "p1"), "s2", "p2");
  const closed = withoutPaneClosed(panes, "p2");
  assert.deepEqual(closed.sessions.map((s) => s.id), ["s1"]);
  assert.equal(closed.active, "s1");
});

test("the last pane of all cannot be closed", () => {
  const panes = firstPanes("s1", "p1");
  assert.equal(withoutPaneClosed(panes, "p1"), panes);
});

test("closing the focused pane of a split moves the focus to its neighbor", () => {
  const split = withSplitFocused(firstPanes("s1", "p1"), "p2", "row");
  const closed = withoutPaneClosed(split, "p2");
  assert.equal(activeSession(closed).focused, "p1");
  assert.deepEqual(activeSession(closed).layout, { kind: "pane", id: "p1" });
});

test("a tab shows the first user message of the pane's session, cut to 30 characters", () => {
  const split = withSplitFocused(firstPanes("s1", "p1"), "p2", "column");
  const said = (text: string) => ({ role: "user" as const, text, streaming: false });
  const items = [{ role: "assistant" as const, text: "Hi", streaming: false }, said("  Fix the\n login   form so it accepts emails"), said("Later")];
  const names = paneNames({ p1: { items: [] }, p2: { items } }, ["p1", "p2", "p3"]);
  assert.deepEqual(names, { p2: "Fix the login form so it accep" });
  assert.equal(tabName(split.sessions[0], names), "Fix the login form so it accep");
  assert.equal(tabName(split.sessions[0], {}), "Session 1");
});
