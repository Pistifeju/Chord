// Tests for the split layout rules.
import assert from "node:assert/strict";
import { test } from "node:test";
import type { Layout } from "../../ipc/bindings.ts";
import { paneIds, resizedSizes, withEvenSizes, withoutPane, withSplit } from "./layout.ts";

const pane = (id: string): Layout => ({ kind: "pane", id });

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
  assert.deepEqual(withoutPane(three, "c"), { kind: "split", direction: "row", sizes: [2 / 3, 1 / 3], children: [pane("a"), pane("b")] });
  assert.deepEqual(withoutPane(withSplit(pane("a"), "a", "b", "row"), "a"), pane("b"));
});

test("a divider moves the two panes beside it and stops at the minimum", () => {
  assert.deepEqual(resizedSizes([0.5, 0.5], 0, 0.2).map((n) => Math.round(n * 100) / 100), [0.7, 0.3]);
  assert.deepEqual(resizedSizes([0.5, 0.3, 0.2], 1, 0.9).map((n) => Math.round(n * 100) / 100), [0.5, 0.4, 0.1]);
});

test("double-click evens out the sizes of one split", () => {
  const uneven: Layout = { kind: "split", direction: "row", sizes: [0.7, 0.2, 0.1], children: [pane("a"), pane("b"), pane("c")] };
  assert.deepEqual(withEvenSizes(uneven, []), { ...uneven, sizes: [1 / 3, 1 / 3, 1 / 3] });
});
