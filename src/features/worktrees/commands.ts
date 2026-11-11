// The project and worktree commands for the commands table.
import type { AppCommand } from "../../app/commands";
import { openNewWorktree } from "./newWorktree";
import { pickProject } from "./store";

export const worktreeCommands: AppCommand[] = [
  { id: "project.add", title: "Add Project…", shortcut: { key: "o", cmd: true }, run: pickProject },
  { id: "worktree.new", title: "New Worktree…", shortcut: { key: "n", cmd: true }, run: openNewWorktree },
];
