// A Files tree entry renamed, created or moved to the Trash: the change on disk, then every file tab and tree path that held it.
import { closeSheet, sheet, sheetOf } from "../../app/sheet";
import { commands } from "../../ipc/bindings";
import { showFile } from "../editor/showing";
import { flushAllSaves } from "../editor/store";
import { closeFilePanesAt, moveFilePanes } from "../panes/store";
import { fail } from "../worktrees/store";
import { joinPath, parentFolder } from "./paths";
import { moveTreeEntries, nameEdit, reloadFiles } from "./store";

function entryMoved(worktree: string, from: string, to: string) {
  moveFilePanes(worktree, from, to);
  moveTreeEntries(worktree, from, to);
}

function entryGone(worktree: string, path: string) {
  closeFilePanesAt(worktree, path);
}

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
      entryMoved(worktree, edit.path, to);
    } else {
      const path = joinPath(edit.folder, name);
      await commands.createFile(worktree, path, edit.kind === "newFolder");
      // A pane can still show a file of this name deleted outside the app; a second editor would save over it.
      if (edit.kind === "newFile") showFile(worktree, path, false);
    }
  } catch (error) {
    fail(error);
  }
  await reloadFiles(worktree);
}

export async function confirmTrash() {
  const pending = sheetOf(sheet.get(), "trashFile");
  closeSheet("trashFile");
  if (!pending) return;
  const { worktree, path } = pending;
  try {
    await flushAllSaves();
    await commands.trashFile(worktree, path);
    entryGone(worktree, path);
  } catch (error) {
    fail(error);
  }
  await reloadFiles(worktree);
}
