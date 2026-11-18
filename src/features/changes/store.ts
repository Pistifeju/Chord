// Connects the Changes view to IPC: each worktree's changed files read from git, one read at a time, a failed read leaving the last list, and a change's diff opened in a tab.
import { commands, type Change } from "../../ipc/bindings";
import { externalStore } from "../../ui/externalStore";
import { fail } from "../../ui/alert";
import { openFileTab } from "../panes/store";

/** Each worktree's changes; a new list per read, which an open diff follows. */
export const changes = externalStore<Record<string, Change[]>>({});

const reading = new Set<string>();

export async function readChanges(worktree: string) {
  if (reading.has(worktree)) return;
  reading.add(worktree);
  try {
    const list = await commands.listChanges(worktree);
    changes.set({ ...changes.get(), [worktree]: list });
  } catch (error) {
    fail(error);
  } finally {
    reading.delete(worktree);
  }
}

export const showDiff = (worktree: string, change: Change, preview = true) =>
  openFileTab(worktree, change.path, preview, change.staged ? "staged" : "unstaged");
