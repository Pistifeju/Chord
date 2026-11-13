// Tests for the tab and tab group rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import type { Layout, WorktreePanes } from "../../ipc/bindings.ts";
import { firstPanes, paneNames, shownSession, tabName, withFileOpened, withFocus, withNewSession, withSplitFocused, withTabClosed, withTabMoved } from "./sessions.ts";

const chat = (id: string, name: string) => ({ kind: "chat" as const, id, name });
const group = (id: string, tabs: ReturnType<typeof chat>[], active: string): Layout => ({ kind: "group", id, tabs, active });

// Two groups side by side: g1 holds p1 and p2, p2 open; g2 holds p3, which has the focus.
const sideBySide: WorktreePanes = {
  layout: {
    kind: "split",
    direction: "row",
    sizes: [0.5, 0.5],
    children: [group("g1", [chat("p1", "Session 1"), chat("p2", "Session 2")], "p2"), group("g2", [chat("p3", "Session 3")], "p3")],
  },
  focused: "p3",
};

test("a new session is named after the highest number and opens in the focused group", () => {
  const two = withNewSession(firstPanes("g1", "p1"), "p2");
  const three = withNewSession(two, "p3");
  assert.deepEqual(three, { layout: group("g1", [chat("p1", "Session 1"), chat("p2", "Session 2"), chat("p3", "Session 3")], "p3"), focused: "p3" });
});

test("a split puts a new group with a new session beside the focused group", () => {
  const split = withSplitFocused(firstPanes("g1", "p1"), "g2", "p2", "row");
  assert.deepEqual(split, {
    layout: { kind: "split", direction: "row", sizes: [0.5, 0.5], children: [group("g1", [chat("p1", "Session 1")], "p1"), group("g2", [chat("p2", "Session 2")], "p2")] },
    focused: "p2",
  });
});

test("focusing a tab opens it in its group", () => {
  const focused = withFocus(sideBySide, "p1");
  assert.equal(focused.focused, "p1");
  assert.deepEqual((focused.layout as Extract<Layout, { kind: "split" }>).children[0], group("g1", [chat("p1", "Session 1"), chat("p2", "Session 2")], "p1"));
});

test("closing a group's only tab closes the group and focuses the group beside it", () => {
  assert.deepEqual(withTabClosed(sideBySide, "p3"), { layout: group("g1", [chat("p1", "Session 1"), chat("p2", "Session 2")], "p2"), focused: "p2" });
});

test("closing the open tab opens the tab beside it and keeps the focus elsewhere", () => {
  const closed = withTabClosed(sideBySide, "p2");
  assert.equal(closed.focused, "p3");
  assert.deepEqual((closed.layout as Extract<Layout, { kind: "split" }>).children[0], group("g1", [chat("p1", "Session 1")], "p1"));
});

test("the last tab of all cannot be closed", () => {
  const panes = firstPanes("g1", "p1");
  assert.equal(withTabClosed(panes, "p1"), panes);
});

test("a tab dropped on another group's bar joins it, and a group left empty goes", () => {
  assert.deepEqual(withTabMoved(sideBySide, "p3", { group: "g1", edge: null }, "new"), {
    layout: group("g1", [chat("p1", "Session 1"), chat("p2", "Session 2"), chat("p3", "Session 3")], "p3"),
    focused: "p3",
  });
});

test("a tab dropped on its own group's edge splits it out; a group's only tab stays", () => {
  const moved = withTabMoved(sideBySide, "p1", { group: "g1", edge: "top" }, "g3");
  assert.deepEqual((moved.layout as Extract<Layout, { kind: "split" }>).children[0], {
    kind: "split",
    direction: "column",
    sizes: [0.5, 0.5],
    children: [group("g3", [chat("p1", "Session 1")], "p1"), group("g1", [chat("p2", "Session 2")], "p2")],
  });
  assert.equal(moved.focused, "p1");
  assert.equal(withTabMoved(sideBySide, "p3", { group: "g2", edge: "left" }, "g3"), sideBySide);
});

test("a file opens in a tab of the focused group", () => {
  const opened = withFileOpened(sideBySide, { kind: "file", id: "f1", path: "src/a.ts" });
  assert.equal(opened.focused, "f1");
  assert.deepEqual((opened.layout as Extract<Layout, { kind: "split" }>).children[1], {
    kind: "group",
    id: "g2",
    tabs: [chat("p3", "Session 3"), { kind: "file", id: "f1", path: "src/a.ts" }],
    active: "f1",
  });
});

test("a tab shows the first user message of its session, cut to 30 characters", () => {
  const said = (text: string) => ({ role: "user" as const, text, streaming: false });
  const items = [{ role: "assistant" as const, text: "Hi", streaming: false }, said("  Fix the\n login   form so it accepts emails"), said("Later")];
  const names = paneNames({ p1: { items: [] }, p2: { items } }, ["p1", "p2", "p3"]);
  assert.deepEqual(names, { p2: "Fix the login form so it accep" });
  assert.equal(tabName(chat("p2", "Session 2"), names), "Fix the login form so it accep");
  assert.equal(tabName(chat("p1", "Session 1"), names), "Session 1");
  assert.equal(tabName({ kind: "file", id: "f1", path: "src/a.ts" }, names), "a.ts");
});

test("the window shows the selected worktree's panes, and none under Settings", () => {
  const all = { "/w": sideBySide };
  assert.equal(shownSession(all, "/w", false), sideBySide);
  assert.equal(shownSession(all, "/w", true), null);
  assert.equal(shownSession(all, null, false), null);
  assert.equal(shownSession(all, "/other", false), null);
});
