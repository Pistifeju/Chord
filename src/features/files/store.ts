// Connects the Files tree rules to IPC: one tree per worktree, its folders read when opened, the right-click changes to its entries, and the file list Open File by Name picks from.
import { closeSheet, openSheet, sheet, sheetOf } from "../../app/sheet";
import { commands } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { movedPath } from "../editor/openFiles";
import { closeFilesAt, flushAllSaves, moveFiles, openFile, showSessions } from "../editor/store";
import { closeFilePanesAt, filePaneOf, focusPane, moveFilePanes } from "../panes/store";
import { fail } from "../worktrees/store";
import { emptyTree, joinPath, parentFolder, type Tree } from "./tree";

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

// A file a pane already shows focuses that pane; any other opens in a tab.
export function showFile(worktree: string, path: string) {
  const pane = filePaneOf(worktree, path);
  if (!pane) return openFile(worktree, path);
  selectFile(worktree, path);
  focusPane(worktree, pane.id);
  showSessions(worktree);
}

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

/** The entry whose name the tree shows as a field: a rename, or a new file or folder inside `folder`. */
export type NameEdit = { worktree: string } & ({ kind: "rename"; path: string } | { kind: "newFile" | "newFolder"; folder: string });

export const nameEdit = externalStore<NameEdit | null>(null);

export function startNew(worktree: string, folder: string, kind: "newFile" | "newFolder") {
  if (folder && !treeOf(worktree).open.includes(folder)) toggleFolder(worktree, folder);
  nameEdit.set({ worktree, kind, folder });
}

export const startRename = (worktree: string, path: string) => nameEdit.set({ worktree, kind: "rename", path });

// An empty or unchanged name cancels.
export async function finishNameEdit(name: string) {
  const edit = nameEdit.get();
  nameEdit.set(null);
  name = name.trim();
  if (!edit || !name) return;
  const { worktree } = edit;
  try {
    if (edit.kind === "rename") {
      const to = joinPath(parentFolder(edit.path), name);
      if (to === edit.path) return;
      // Pending edits land under the old name first, so no save goes to a path that is gone.
      await flushAllSaves();
      await commands.renameFile(worktree, edit.path, to);
      moveFiles(worktree, edit.path, to);
      moveFilePanes(worktree, edit.path, to);
      const moved = (path: string) => movedPath(path, edit.path, to);
      update(worktree, (tree) => ({ ...tree, open: tree.open.map(moved), selected: tree.selected && moved(tree.selected) }));
    } else {
      const path = joinPath(edit.folder, name);
      await commands.createFile(worktree, path, edit.kind === "newFolder");
      if (edit.kind === "newFile") openFile(worktree, path);
    }
  } catch (error) {
    fail(error);
  }
  await reloadFiles(worktree);
}

export const requestTrash = (worktree: string, path: string) => openSheet({ kind: "trashFile", worktree, path });

export async function confirmTrash() {
  const pending = sheetOf(sheet.get(), "trashFile");
  closeSheet("trashFile");
  if (!pending) return;
  const { worktree, path } = pending;
  try {
    await flushAllSaves();
    await commands.trashFile(worktree, path);
    closeFilesAt(worktree, path);
    closeFilePanesAt(worktree, path);
  } catch (error) {
    fail(error);
  }
  await reloadFiles(worktree);
}

export const revealFile = (worktree: string, path: string) => commands.revealFile(worktree, path).catch(fail);

// The worktree's files while Open File by Name shows; null until they are read.
export const namedFiles = externalStore<string[] | null>(null);

export async function openFileByName(worktree: string) {
  namedFiles.set(null);
  openSheet({ kind: "openFileByName", worktree });
  try {
    const files = await commands.listFiles(worktree);
    if (sheetOf(sheet.get(), "openFileByName")?.worktree === worktree) namedFiles.set(files);
  } catch (error) {
    closeSheet("openFileByName");
    fail(error);
  }
}

export function pickFileByName(worktree: string, path: string) {
  closeSheet("openFileByName");
  showFile(worktree, path);
}
