// Tests for the split layout rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import type { Layout } from "../../ipc/bindings.ts";
import { dropEdge, dropPreview, paneIds, resizedSizes, withBeside, withEvenSizes, withoutGroup, type Split } from "./layout.ts";

const pane = (id: string): Layout => ({ kind: "group", id, tabs: [{ kind: "chat", id, name: "Session 1" }], active: id });
const withSplit = (layout: Layout, at: string, created: string, direction: "row" | "column") => withBeside(layout, at, pane(created), direction, false);

test("splitting a lone pane makes two halves", () => {
  assert.deepEqual(withSplit(pane("a"), "a", "b", "row"), { kind: "split", direction: "row", sizes: [0.5, 0.5], children: [pane("a"), pane("b")] });
});

test("splitting in the same direction adds a sibling that takes half of the pane's share", () => {
  const two = withSplit(pane("a"), "a", "b", "row");
  assert.deepEqual(withSplit(two, "a", "c", "row"), { kind: "split", direction: "row", sizes: [0.25, 0.25, 0.5], children: [pane("a"), pane("c"), pane("b")] });
});

test("splitting across the direction nests", () => {
  const two = withSplit(pane("a"), "a", "b", "row");
  const nested = withSplit(two, "b", "c", "column");
  assert.deepEqual(paneIds(nested), ["a", "b", "c"]);
  assert.deepEqual((nested as Extract<Layout, { kind: "split" }>).children[1], { kind: "split", direction: "column", sizes: [0.5, 0.5], children: [pane("b"), pane("c")] });
});

test("closing a pane gives its share to the others and collapses a split of one", () => {
  const three = withSplit(withSplit(pane("a"), "a", "b", "row"), "b", "c", "row");
  assert.deepEqual(withoutGroup(three, "c"), { kind: "split", direction: "row", sizes: [2 / 3, 1 / 3], children: [pane("a"), pane("b")] });
  assert.deepEqual(withoutGroup(withSplit(pane("a"), "a", "b", "row"), "a"), pane("b"));
});

test("a divider moves the two panes beside it and stops at the minimum", () => {
  assert.deepEqual(resizedSizes([0.5, 0.5], 0, 0.2, [0.1, 0.1]).map((n) => Math.round(n * 100) / 100), [0.7, 0.3]);
  assert.deepEqual(resizedSizes([0.5, 0.3, 0.2], 1, 0.9, [0.1, 0.1]).map((n) => Math.round(n * 100) / 100), [0.5, 0.4, 0.1]);
});

test("a pane already below its minimum keeps its size and can only grow", () => {
  assert.deepEqual(resizedSizes([0.9, 0.1], 0, 0.05, [0.3, 0.3]).map((n) => Math.round(n * 100) / 100), [0.9, 0.1]);
  assert.deepEqual(resizedSizes([0.9, 0.1], 0, -0.2, [0.3, 0.3]).map((n) => Math.round(n * 100) / 100), [0.7, 0.3]);
});

test("double-click evens out the sizes of one split", () => {
  const uneven: Layout = { kind: "split", direction: "row", sizes: [0.7, 0.2, 0.1], children: [pane("a"), pane("b"), pane("c")] };
  assert.deepEqual(withEvenSizes(uneven, []), { ...uneven, sizes: [1 / 3, 1 / 3, 1 / 3] });
});

test("a point in a pane picks the nearest edge", () => {
  const box = { left: 100, top: 0, width: 400, height: 200 };
  assert.equal(dropEdge(110, 100, box), "left");
  assert.equal(dropEdge(490, 100, box), "right");
  assert.equal(dropEdge(300, 10, box), "top");
  assert.equal(dropEdge(300, 195, box), "bottom");
  assert.equal(dropEdge(150, 100, box), "left");
  assert.equal(dropEdge(300, 60, box), "top");
});

test("the drop preview is the half of the pane on the dropped edge", () => {
  const box = { left: 100, top: 20, width: 400, height: 200 };
  assert.deepEqual(dropPreview("left", box), { left: 100, top: 20, width: 200, height: 200 });
  assert.deepEqual(dropPreview("right", box), { left: 300, top: 20, width: 200, height: 200 });
  assert.deepEqual(dropPreview("top", box), { left: 100, top: 20, width: 400, height: 100 });
  assert.deepEqual(dropPreview("bottom", box), { left: 100, top: 120, width: 400, height: 100 });
});

test("a layout dropped before a pane goes first and halves the pane's share", () => {
  const layout: Layout = { kind: "split", direction: "row", sizes: [0.5, 0.5], children: [pane("a"), pane("b")] };
  const next = withBeside(layout, "b", pane("c"), "row", true) as Split;
  assert.deepEqual(next.children.map((c) => (c.kind === "group" ? c.id : "?")), ["a", "c", "b"]);
  assert.deepEqual(next.sizes, [0.5, 0.25, 0.25]);
});
