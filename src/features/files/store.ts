import { closeSheet, openSheet, sheet, sheetOf } from "../../app/sheet";
import { commands } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { fail } from "../../ui/alert";
import { emptyTree, withTreeMoved, type Tree } from "./tree";
import { recentFirst } from "./openByName";

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

export const moveTreeEntries = (worktree: string, from: string, to: string) => update(worktree, (tree) => withTreeMoved(tree, from, to));

export const collapseAll = (worktree: string) => update(worktree, (tree) => ({ ...tree, open: [] }));

export const setFilter = (worktree: string, filter: string) => update(worktree, (tree) => ({ ...tree, filter }));

export async function reloadFiles(worktree: string) {
  const folders = ["", ...treeOf(worktree).open];
  const results = await Promise.allSettled(folders.map((folder) => commands.listFolder(worktree, folder)));
  const root = results[0];
  if (root.status === "rejected") {
    fail(root.reason);
    return;
  }
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

type NameEdit = { worktree: string } & ({ kind: "rename"; path: string } | { kind: "newFile" | "newFolder"; folder: string });

export const nameEdit = externalStore<NameEdit | null>(null);

export function startNew(worktree: string, folder: string, kind: "newFile" | "newFolder") {
  if (folder && !treeOf(worktree).open.includes(folder)) toggleFolder(worktree, folder);
  nameEdit.set({ worktree, kind, folder });
}

export const startRename = (worktree: string, path: string) => nameEdit.set({ worktree, kind: "rename", path });

export const requestTrash = (worktree: string, path: string) => openSheet({ kind: "trashFile", worktree, path });

export const revealFile = (worktree: string, path: string) => commands.revealFile(worktree, path).catch(fail);

const opened: Record<string, string[]> = {};

export function noteOpened(worktree: string, path: string) {
  opened[worktree] = [path, ...(opened[worktree] ?? []).filter((at) => at !== path)];
}

export const namedFiles = externalStore<string[] | null>(null);

export async function openFileByName(worktree: string) {
  namedFiles.set(null);
  openSheet({ kind: "openFileByName", worktree });
  try {
    const files = await commands.listFiles(worktree);
    if (sheetOf(sheet.get(), "openFileByName")?.worktree === worktree) namedFiles.set(recentFirst(files, opened[worktree] ?? []));
  } catch (error) {
    if (sheetOf(sheet.get(), "openFileByName")?.worktree === worktree) closeSheet("openFileByName");
    fail(error);
  }
}
