// Rules for one split layout: split a pane, close one, its file panes, and the sizes of a split's children.
import type { Direction, Layout } from "../../ipc/bindings";
import { isAtOrInside, movedPath } from "../files/paths.ts";

export type Split = Extract<Layout, { kind: "split" }>;
export type FilePane = Extract<Layout, { kind: "file" }>;

/** The share a child keeps at least, so a divider never hides a pane. */
export const MIN_SIZE = 0.1;

export const paneIds = (layout: Layout): string[] => (layout.kind === "split" ? layout.children.flatMap(paneIds) : [layout.id]);

export const chatPaneIds = (layout: Layout): string[] =>
  layout.kind === "split" ? layout.children.flatMap(chatPaneIds) : layout.kind === "pane" ? [layout.id] : [];

export const filePanes = (layout: Layout): FilePane[] => (layout.kind === "split" ? layout.children.flatMap(filePanes) : layout.kind === "file" ? [layout] : []);

// A rename moves the file panes of the entry and of everything inside it to their new paths.
export function withFilePanesMoved(layout: Layout, from: string, to: string): Layout {
  if (layout.kind === "split") return { ...layout, children: layout.children.map((child) => withFilePanesMoved(child, from, to)) };
  return layout.kind === "file" && isAtOrInside(layout.path, from) ? { ...layout, path: movedPath(layout.path, from, to) } : layout;
}

const evenSizes = (count: number): number[] => Array.from({ length: count }, () => 1 / count);

// `inserted` goes before or after `pane` and takes half of its share; a lone pane in a same-direction split gains a sibling, anything else nests.
export function withBeside(layout: Layout, pane: string, inserted: Layout, direction: Direction, before: boolean): Layout {
  if (layout.kind !== "split") {
    if (layout.id !== pane) return layout;
    return { kind: "split", direction, sizes: [0.5, 0.5], children: before ? [inserted, layout] : [layout, inserted] };
  }
  const at = layout.children.findIndex((child) => child.kind !== "split" && child.id === pane);
  if (at >= 0 && layout.direction === direction) {
    const sizes = layout.sizes.map(Number);
    const half = sizes[at] / 2;
    sizes.splice(at, 1, half, half);
    const children = [...layout.children];
    children.splice(before ? at : at + 1, 0, inserted);
    return { ...layout, sizes, children };
  }
  return { ...layout, children: layout.children.map((child) => withBeside(child, pane, inserted, direction, before)) };
}

export type Box = { left: number; top: number; width: number; height: number };

export type Edge = "left" | "right" | "top" | "bottom";

// The pane edge the point is nearest to, measured in shares of the pane's width and height.
export function dropEdge(x: number, y: number, box: Box): Edge {
  const fx = (x - box.left) / box.width;
  const fy = (y - box.top) / box.height;
  const near = [
    { edge: "left" as const, share: fx },
    { edge: "right" as const, share: 1 - fx },
    { edge: "top" as const, share: fy },
    { edge: "bottom" as const, share: 1 - fy },
  ].reduce((a, b) => (b.share < a.share ? b : a));
  return near.edge;
}

export const edgeDirection = (edge: Edge): Direction => (edge === "left" || edge === "right" ? "row" : "column");

// The half of the pane that a drop on `edge` would fill.
export function dropPreview(edge: Edge, box: Box): Box {
  const row = edgeDirection(edge) === "row";
  return {
    left: edge === "right" ? box.left + box.width / 2 : box.left,
    top: edge === "bottom" ? box.top + box.height / 2 : box.top,
    width: row ? box.width / 2 : box.width,
    height: row ? box.height : box.height / 2,
  };
}

// The new pane goes after `pane` and takes half of its share.
export const withSplit = (layout: Layout, pane: string, created: string, direction: Direction): Layout =>
  withBeside(layout, pane, { kind: "pane", id: created }, direction, false);

// The closed pane's share goes to the others in proportion; a split left with one child becomes that child.
export function withoutPane(layout: Layout, pane: string): Layout | null {
  if (layout.kind !== "split") return layout.id === pane ? null : layout;
  const kept: { child: Layout; size: number }[] = [];
  layout.children.forEach((child, i) => {
    const next = withoutPane(child, pane);
    if (next) kept.push({ child: next, size: Number(layout.sizes[i]) });
  });
  if (kept.length === 0) return null;
  if (kept.length === 1) return kept[0].child;
  const total = kept.reduce((sum, k) => sum + k.size, 0);
  return { ...layout, sizes: kept.map((k) => k.size / total), children: kept.map((k) => k.child) };
}

// The split at `path` (child indexes from the root) gets `change` applied.
function withSplitAt(layout: Layout, path: number[], change: (split: Split) => Split): Layout {
  if (layout.kind !== "split") return layout;
  if (path.length === 0) return change(layout);
  const [first, ...rest] = path;
  return { ...layout, children: layout.children.map((child, i) => (i === first ? withSplitAt(child, rest, change) : child)) };
}

export const withSizes = (layout: Layout, path: number[], sizes: number[]): Layout => withSplitAt(layout, path, (split) => ({ ...split, sizes }));

export const withEvenSizes = (layout: Layout, path: number[]): Layout => withSplitAt(layout, path, (split) => ({ ...split, sizes: evenSizes(split.children.length) }));

// Moves the divider after child `index` by `delta` (a share of the split); the two children it touches keep MIN_SIZE.
export function resizedSizes(sizes: number[], index: number, delta: number): number[] {
  const pair = sizes[index] + sizes[index + 1];
  const first = Math.min(Math.max(sizes[index] + delta, MIN_SIZE), pair - MIN_SIZE);
  return sizes.map((size, i) => (i === index ? first : i === index + 1 ? pair - first : size));
}
