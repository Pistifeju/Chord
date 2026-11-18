// Connects the Changes view to IPC: each worktree's changed files read from git, one read at a time, a failed read leaving the last list, a change's diff opened in a tab, and the stage, unstage and discard of files and hunks, each followed by a read.
import { commands, type Change, type Hunk, type HunkAction } from "../../ipc/bindings";
import { closeSheet, openSheet, sheet, sheetOf } from "../../app/sheet";
import { externalStore } from "../../ui/externalStore";
import { fail } from "../../ui/alert";
import { flushAllSaves } from "../editor/store";
import { closeFilePanesAt, openFileTab } from "../panes/store";

/** Each worktree's changes; a new list per read, which an open diff follows. */
export const changes = externalStore<Record<string, Change[]>>({});

// Each worktree with a read in flight, and whether another read was asked for meanwhile.
const reading = new Map<string, boolean>();

// The worktrees Chord staged, unstaged or discarded in since their last read began, and the lists read after that.
const acted = new Set<string>();
const actedLists = new WeakSet<Change[]>();

/** Whether the list came from a read after Chord's own stage, unstage or discard, so the selection may move on. */
export const followsAct = (list: Change[]) => actedLists.has(list);

export async function readChanges(worktree: string) {
  if (reading.has(worktree)) return void reading.set(worktree, true);
  reading.set(worktree, false);
  const afterAct = acted.delete(worktree);
  try {
    const list = await commands.listChanges(worktree);
    if (afterAct) actedLists.add(list);
    changes.set({ ...changes.get(), [worktree]: list });
  } catch (error) {
    fail(error);
  } finally {
    const again = reading.get(worktree);
    reading.delete(worktree);
    if (again) void readChanges(worktree);
  }
}

export const showDiff = (worktree: string, change: Change, preview = true) => openFileTab(worktree, change.path, preview, change.staged ? "staged" : "unstaged");

const pathsOf = (list: Change[]) => list.map((c) => c.path);

async function act(worktree: string, run: () => Promise<unknown>) {
  try {
    // A pending edit would land after git acts: staged without it, or brought back after a discard.
    await flushAllSaves();
    await run();
  } catch (error) {
    fail(error);
  }
  acted.add(worktree);
  await readChanges(worktree);
}

export const stageChanges = (worktree: string, list: Change[]) => act(worktree, () => commands.stageFiles(worktree, pathsOf(list)));

// A rename's old path goes back into the index with its new one.
const indexPaths = (list: Change[]) => list.flatMap((c) => (c.oldPath ? [c.path, c.oldPath] : [c.path]));

export const unstageChanges = (worktree: string, list: Change[]) => act(worktree, () => commands.unstageFiles(worktree, indexPaths(list)));

export const applyHunk = (worktree: string, path: string, hunk: Hunk, action: Exclude<HunkAction, "discard" | "discardStaged">) => act(worktree, () => commands.applyHunk(worktree, path, hunk, action));

/** Asks before the changes, or with `hunk` that hunk of the one change, are discarded. */
export const requestDiscard = (worktree: string, list: Change[], hunk?: Hunk) => openSheet({ kind: "discardChanges", worktree, changes: list, ...(hunk && { hunk }) });

/** Whether discarding the change moves its file to the Trash: git holds no copy of it to restore. */
export const goesToTrash = (c: Change) => c.status === "untracked" || (c.staged && c.status === "added");

export async function confirmDiscard() {
  const pending = sheetOf(sheet.get(), "discardChanges");
  closeSheet("discardChanges");
  if (!pending) return;
  const { worktree, changes: list, hunk } = pending;
  await act(worktree, async () => {
    // git holds no copy of a new file, so its hunk goes the file's way, to the Trash.
    if (hunk && !goesToTrash(list[0])) await commands.applyHunk(worktree, list[0].path, hunk, list[0].staged ? "discardStaged" : "discard");
    else if (list[0].staged) {
      // Unstaged first, the files go back to HEAD with the restore from the index.
      await commands.unstageFiles(worktree, indexPaths(list));
      await commands.discardFiles(worktree, indexPaths(list));
    } else await commands.discardFiles(worktree, pathsOf(list));
    // A file that went to the Trash loses its tabs, as in the Files tree.
    for (const change of list) if (goesToTrash(change)) closeFilePanesAt(worktree, change.path);
  });
}
