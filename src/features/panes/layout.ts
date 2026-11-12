// Rules for one split layout: split a pane, close one, and the sizes of a split's children.
import type { Direction, Layout } from "../../ipc/bindings";

export type Split = Extract<Layout, { kind: "split" }>;

/** The share a child keeps at least, so a divider never hides a pane. */
export const MIN_SIZE = 0.1;

export const paneIds = (layout: Layout): string[] => (layout.kind === "pane" ? [layout.id] : layout.children.flatMap(paneIds));

const evenSizes = (count: number): number[] => Array.from({ length: count }, () => 1 / count);

// The new pane goes after `pane` and takes half of its share; a split in the same direction grows instead of nesting.
export function withSplit(layout: Layout, pane: string, created: string, direction: Direction): Layout {
  if (layout.kind === "pane") {
    if (layout.id !== pane) return layout;
    return { kind: "split", direction, sizes: [0.5, 0.5], children: [layout, { kind: "pane", id: created }] };
  }
  const at = layout.children.findIndex((child) => child.kind === "pane" && child.id === pane);
  if (at >= 0 && layout.direction === direction) {
    const sizes = layout.sizes.map(Number);
    const half = sizes[at] / 2;
    sizes.splice(at, 1, half, half);
    const children = [...layout.children];
    children.splice(at + 1, 0, { kind: "pane", id: created });
    return { ...layout, sizes, children };
  }
  return { ...layout, children: layout.children.map((child) => withSplit(child, pane, created, direction)) };
}

// The closed pane's share goes to the others in proportion; a split left with one child becomes that child.
export function withoutPane(layout: Layout, pane: string): Layout | null {
  if (layout.kind === "pane") return layout.id === pane ? null : layout;
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
  if (layout.kind === "pane") return layout;
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
