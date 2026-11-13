// Rules for the Files tree: which rows show for the loaded folders, the open ones and the filter.
import type { FileEntry } from "../../ipc/bindings.ts";

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
