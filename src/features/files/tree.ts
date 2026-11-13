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

/** The folder that holds `path`; "" for the worktree root. */
export const parentFolder = (path: string) => path.slice(0, Math.max(path.lastIndexOf("/"), 0));

export const joinPath = (folder: string, name: string) => (folder ? `${folder}/${name}` : name);

// The length of a name without its extension; a dotfile like `.env` is all name.
export function stemLength(name: string): number {
  const dot = name.lastIndexOf(".");
  return dot > 0 ? dot : name.length;
}
