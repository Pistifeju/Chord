// Connects the Files tree rules to IPC: one tree per worktree, its folders read when opened.
import { commands } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { fail } from "../worktrees/store";
import { emptyTree, type Tree } from "./tree";

export const trees = externalStore<Record<string, Tree>>({});

const treeOf = (worktree: string): Tree => trees.get()[worktree] ?? emptyTree;
const update = (worktree: string, change: (tree: Tree) => Tree) => trees.set({ ...trees.get(), [worktree]: change(treeOf(worktree)) });

async function loadFolder(worktree: string, folder: string) {
  try {
    const entries = await commands.listFolder(worktree, folder);
    update(worktree, (tree) => ({ ...tree, children: { ...tree.children, [folder]: entries } }));
  } catch (error) {
    fail(error);
  }
}

export function loadRoot(worktree: string) {
  if (!treeOf(worktree).children[""]) void loadFolder(worktree, "");
}

export function toggleFolder(worktree: string, folder: string) {
  const tree = treeOf(worktree);
  if (tree.open.includes(folder)) {
    update(worktree, (t) => ({ ...t, open: t.open.filter((path) => path !== folder) }));
    return;
  }
  update(worktree, (t) => ({ ...t, open: [...t.open, folder] }));
  if (!tree.children[folder]) void loadFolder(worktree, folder);
}

export const selectFile = (worktree: string, path: string) => update(worktree, (tree) => ({ ...tree, selected: path }));

export const collapseAll = (worktree: string) => update(worktree, (tree) => ({ ...tree, open: [] }));

export const setFilter = (worktree: string, filter: string) => update(worktree, (tree) => ({ ...tree, filter }));

// Reads the root and every open folder again; an open folder that is gone closes. Selection and the other open folders stay.
export async function reloadFiles(worktree: string) {
  const folders = ["", ...treeOf(worktree).open];
  const results = await Promise.allSettled(folders.map((folder) => commands.listFolder(worktree, folder)));
  const root = results[0];
  if (root.status === "rejected") {
    fail(root.reason);
    return;
  }
  // The open list is read again here: a Collapse All or a toggle during the reload wins.
  update(worktree, (tree) => {
    const children: Tree["children"] = {};
    results.forEach((result, i) => {
      if (result.status === "fulfilled") children[folders[i]] = result.value;
    });
    const open = tree.open.filter((path) => (folders.includes(path) ? path in children : true));
    for (const path of open) if (!(path in children) && tree.children[path]) children[path] = tree.children[path];
    return { ...tree, children, open };
  });
}
