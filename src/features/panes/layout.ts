import type { Direction, Layout, Tab } from "../../ipc/bindings";
import { isAtOrInside, movedPath } from "../files/paths.ts";

export type Split = Extract<Layout, { kind: "split" }>;
export type Group = Extract<Layout, { kind: "group" }>;
export type FilePane = Extract<Tab, { kind: "file" }>;
export type BrowserPane = Extract<Tab, { kind: "browser" }>;

const MIN_GROUP: Record<Direction, number> = { row: 280, column: 200 };

export const groups = (layout: Layout): Group[] => (layout.kind === "split" ? layout.children.flatMap(groups) : [layout]);

const tabs = (layout: Layout): Tab[] => groups(layout).flatMap((group) => group.tabs);

export const paneIds = (layout: Layout): string[] => tabs(layout).map((tab) => tab.id);

type ChatTab = Extract<Tab, { kind: "chat" }>;

export const chatTabs = (layout: Layout): ChatTab[] => tabs(layout).flatMap((tab) => (tab.kind === "chat" ? [tab] : []));

export const chatPaneIds = (layout: Layout): string[] => chatTabs(layout).map((tab) => tab.id);

export const filePanes = (layout: Layout): FilePane[] => tabs(layout).flatMap((tab) => (tab.kind === "file" ? [tab] : []));

export const browserPaneIds = (layout: Layout): string[] => tabs(layout).flatMap((tab) => (tab.kind === "browser" ? [tab.id] : []));

export const shownTabIds = (layout: Layout): string[] => groups(layout).map((group) => group.active);

export const groupOf = (layout: Layout, tab: string): Group | undefined => groups(layout).find((group) => group.tabs.some((t) => t.id === tab));

export function withGroup(layout: Layout, id: string, change: (group: Group) => Group): Layout {
  if (layout.kind === "group") return layout.id === id ? change(layout) : layout;
  return { ...layout, children: layout.children.map((child) => withGroup(child, id, change)) };
}

export function withBrowserPage(layout: Layout, id: string, page: Partial<Pick<BrowserPane, "url" | "title">>): Layout {
  if (layout.kind === "split") return { ...layout, children: layout.children.map((child) => withBrowserPage(child, id, page)) };
  return { ...layout, tabs: layout.tabs.map((tab) => (tab.kind === "browser" && tab.id === id ? { ...tab, ...page } : tab)) };
}

export function withFilePanesMoved(layout: Layout, from: string, to: string): Layout {
  if (layout.kind === "split") return { ...layout, children: layout.children.map((child) => withFilePanesMoved(child, from, to)) };
  return { ...layout, tabs: layout.tabs.map((tab) => (tab.kind === "file" && isAtOrInside(tab.path, from) ? { ...tab, path: movedPath(tab.path, from, to) } : tab)) };
}

const evenSizes = (count: number): number[] => Array.from({ length: count }, () => 1 / count);

export function withBeside(layout: Layout, group: string, inserted: Layout, direction: Direction, before: boolean): Layout {
  if (layout.kind !== "split") {
    if (layout.id !== group) return layout;
    return { kind: "split", direction, sizes: [0.5, 0.5], children: before ? [inserted, layout] : [layout, inserted] };
  }
  const at = layout.children.findIndex((child) => child.kind !== "split" && child.id === group);
  if (at >= 0 && layout.direction === direction) {
    const sizes = layout.sizes.map(Number);
    const half = sizes[at] / 2;
    sizes.splice(at, 1, half, half);
    const children = [...layout.children];
    children.splice(before ? at : at + 1, 0, inserted);
    return { ...layout, sizes, children };
  }
  return { ...layout, children: layout.children.map((child) => withBeside(child, group, inserted, direction, before)) };
}

export type Box = { left: number; top: number; width: number; height: number };

export type Edge = "left" | "right" | "top" | "bottom";

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

export const fitsSplit = (box: Box, direction: Direction): boolean => (direction === "row" ? box.width : box.height) / 2 >= MIN_GROUP[direction];

export function minExtent(layout: Layout, direction: Direction): number {
  if (layout.kind === "group") return MIN_GROUP[direction];
  const mins = layout.children.map((child) => minExtent(child, direction));
  return layout.direction === direction ? mins.reduce((sum, min) => sum + min, 0) : Math.max(...mins);
}

export const edgeDirection = (edge: Edge): Direction => (edge === "left" || edge === "right" ? "row" : "column");

export function dropPreview(edge: Edge, box: Box): Box {
  const row = edgeDirection(edge) === "row";
  return {
    left: edge === "right" ? box.left + box.width / 2 : box.left,
    top: edge === "bottom" ? box.top + box.height / 2 : box.top,
    width: row ? box.width / 2 : box.width,
    height: row ? box.height : box.height / 2,
  };
}

export function withoutGroup(layout: Layout, group: string): Layout | null {
  if (layout.kind !== "split") return layout.id === group ? null : layout;
  const kept: { child: Layout; size: number }[] = [];
  layout.children.forEach((child, i) => {
    const next = withoutGroup(child, group);
    if (next) kept.push({ child: next, size: Number(layout.sizes[i]) });
  });
  if (kept.length === 0) return null;
  if (kept.length === 1) return kept[0].child;
  const total = kept.reduce((sum, k) => sum + k.size, 0);
  return { ...layout, sizes: kept.map((k) => k.size / total), children: kept.map((k) => k.child) };
}

function withSplitAt(layout: Layout, path: number[], change: (split: Split) => Split): Layout {
  if (layout.kind !== "split") return layout;
  if (path.length === 0) return change(layout);
  const [first, ...rest] = path;
  return { ...layout, children: layout.children.map((child, i) => (i === first ? withSplitAt(child, rest, change) : child)) };
}

export const withSizes = (layout: Layout, path: number[], sizes: number[]): Layout => withSplitAt(layout, path, (split) => ({ ...split, sizes }));

export const withEvenSizes = (layout: Layout, path: number[]): Layout => withSplitAt(layout, path, (split) => ({ ...split, sizes: evenSizes(split.children.length) }));

export function resizedSizes(sizes: number[], index: number, delta: number, mins: [number, number]): number[] {
  const pair = sizes[index] + sizes[index + 1];
  const low = Math.min(mins[0], sizes[index]);
  const high = pair - Math.min(mins[1], sizes[index + 1]);
  const first = Math.min(Math.max(sizes[index] + delta, low), high);
  return sizes.map((size, i) => (i === index ? first : i === index + 1 ? pair - first : size));
}
