// The project, worktree and sheet commands for the commands table.
import type { AppCommand } from "../../app/commands";
import { cancelQuit, quitAsked } from "../quit/store";
import { closeNewWorktree, newWorktree, openNewWorktree } from "./newWorktree";
import { worktreeOrder } from "./projects";
import { cancelDeletion, cancelForceRemoval, deletion, forceRemoval, pickProject, projects, removeProject, requestDelete, selectWorktree } from "./store";

function deleteWorktreeAt(path?: string) {
  const { list, worktrees } = projects.get();
  const owner = list.find((p) => worktrees[p.path]?.some((w) => w.path === path));
  if (owner && path) requestDelete(owner.path, path);
}

const sheetOpen = () => newWorktree.get().open || deletion.get() !== null || forceRemoval.get() !== null || quitAsked.get();

function closeSheets() {
  if (newWorktree.get().open) closeNewWorktree();
  cancelDeletion();
  cancelForceRemoval();
  cancelQuit();
}

const selectCommands: AppCommand[] = Array.from({ length: 9 }, (_, i) => ({
  id: `worktree.select.${i + 1}`,
  title: `Select Worktree ${i + 1}`,
  shortcut: { key: String(i + 1), cmd: true },
  run: () => {
    const path = worktreeOrder(projects.get())[i];
    if (path) selectWorktree(path);
  },
}));

export const worktreeCommands: AppCommand[] = [
  { id: "project.add", title: "Add Project", shortcut: { key: "o", cmd: true }, run: pickProject },
  { id: "worktree.new", title: "New Worktree", shortcut: { key: "n", cmd: true }, run: openNewWorktree },
  { id: "project.remove", title: "Remove Project", run: (path) => path && void removeProject(path) },
  { id: "worktree.delete", title: "Delete Worktree", run: deleteWorktreeAt },
  { id: "sheet.close", title: "Close", shortcut: { key: "escape" }, when: sheetOpen, run: closeSheets },
  ...selectCommands,
];
