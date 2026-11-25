import type { AppCommand } from "../../app/commands";
import { closeSheet, sheet } from "../../app/sheet";
import { openNewWorktree } from "./newWorktree";
import { startRename } from "./rename";
import { ownerOf, worktreeOrder } from "./projects";
import { pickProject, projects, requestDelete, requestRemoveProject, selectWorktree } from "./store";

function deleteWorktreeAt(path?: string) {
  const owner = path ? ownerOf(projects.get(), path) : undefined;
  if (owner && path) requestDelete(owner, path);
}

const noSheet = () => sheet.get() === null;

const selectCommands: AppCommand[] = Array.from({ length: 9 }, (_, i) => ({
  id: `worktree.select.${i + 1}`,
  title: `Select Worktree ${i + 1}`,
  shortcut: { key: String(i + 1), cmd: true },
  when: noSheet,
  run: () => {
    const path = worktreeOrder(projects.get())[i];
    if (path) selectWorktree(path);
  },
}));

export const worktreeCommands: AppCommand[] = [
  { id: "project.add", title: "Add Project", shortcut: { key: "o", cmd: true }, when: noSheet, run: pickProject },
  { id: "worktree.new", title: "New Worktree", shortcut: { key: "n", cmd: true }, when: noSheet, run: openNewWorktree },
  { id: "project.remove", title: "Remove Project", run: (path) => path && requestRemoveProject(path) },
  { id: "worktree.rename", title: "Rename", run: startRename },
  { id: "worktree.delete", title: "Delete Worktree", run: deleteWorktreeAt },
  { id: "sheet.close", title: "Close", shortcut: { key: "escape" }, when: () => sheet.get() !== null, run: () => closeSheet() },
  ...selectCommands,
];
