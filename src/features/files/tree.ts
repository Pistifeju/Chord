// Rules for the Files tree: which rows show for the loaded folders, the open ones and the filter.
import type { FileEntry } from "../../ipc/bindings.ts";
import { movedPath } from "./paths.ts";

/** `children` holds each loaded folder's entries by relative path; the worktree root is "". */
export type Tree = { children: Record<string, FileEntry[]>; open: string[]; selected: string | null; filter: string };

export type Row = { entry: FileEntry; depth: number; open: boolean };

export const emptyTree: Tree = { children: {}, open: [], selected: null, filter: "" };

// An entry stays while its name matches or a loaded entry below it does.
function keeps(tree: Tree, entry: FileEntry, needle: string): boolean {
  return entry.name.toLowerCase().includes(needle) || below(tree, entry, needle);
}

const below = (tree: Tree, entry: FileEntry, needle: string): boolean =>
  entry.isDir && (tree.children[entry.path] ?? []).some((child) => keeps(tree, child, needle));

export function visibleRows(tree: Tree): Row[] {
  const needle = tree.filter.trim().toLowerCase();
  const rows: Row[] = [];
  const add = (folder: string, depth: number) => {
    for (const entry of tree.children[folder] ?? []) {
      if (needle && !keeps(tree, entry, needle)) continue;
      const open = entry.isDir && (tree.open.includes(entry.path) || (needle !== "" && below(tree, entry, needle)));
      rows.push({ entry, depth, open });
      if (open) add(entry.path, depth + 1);
    }
  };
  add("", 0);
  return rows;
}

/** What an arrow key on the row at `path` does: move the focus to a row, open or close a folder, or nothing. */
export type TreeKeyAction = { focus: string } | { toggle: string } | null;

export function treeKeyAction(rows: Row[], path: string, key: string): TreeKeyAction {
  const at = rows.findIndex((row) => row.entry.path === path);
  if (at < 0) return null;
  const row = rows[at];
  const focus = (target: Row | undefined) => (target ? { focus: target.entry.path } : null);
  if (key === "ArrowDown") return focus(rows[at + 1]);
  if (key === "ArrowUp") return focus(rows[at - 1]);
  if (key === "ArrowRight") return row.entry.isDir && !row.open ? { toggle: path } : null;
  if (key !== "ArrowLeft") return null;
  if (row.open) return { toggle: path };
  // The parent is the nearest row above one level up; a folder chain row has no row per folder.
  for (let i = at - 1; i >= 0; i--) if (rows[i].depth < row.depth) return focus(rows[i]);
  return null;
}

// A rename keeps the open folders and the selection of the entry and of everything inside it, under their new paths.
export function withTreeMoved(tree: Tree, from: string, to: string): Tree {
  const moved = (path: string) => movedPath(path, from, to);
  return { ...tree, open: tree.open.map(moved), selected: tree.selected && moved(tree.selected) };
}

// The length of a name without its extension; a dotfile like `.env` is all name.
export function stemLength(name: string): number {
  const dot = name.lastIndexOf(".");
  return dot > 0 ? dot : name.length;
}
