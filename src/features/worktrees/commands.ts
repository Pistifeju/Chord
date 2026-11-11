// The project and worktree commands for the commands table.
import type { AppCommand } from "../../app/commands";
import { openNewWorktree } from "./newWorktree";
import { pickProject, projects, removeProject, requestDelete } from "./store";

function deleteWorktreeAt(path?: string) {
  const { list, worktrees } = projects.get();
  const owner = list.find((p) => worktrees[p.path]?.some((w) => w.path === path));
  if (owner && path) requestDelete(owner.path, path);
}

export const worktreeCommands: AppCommand[] = [
  { id: "project.add", title: "Add Project…", shortcut: { key: "o", cmd: true }, run: pickProject },
  { id: "worktree.new", title: "New Worktree…", shortcut: { key: "n", cmd: true }, run: openNewWorktree },
  { id: "project.remove", title: "Remove Project", run: (path) => path && void removeProject(path) },
  { id: "worktree.delete", title: "Delete Worktree", run: deleteWorktreeAt },
];
