// The Changes view's commands for the commands table: show or hide the view.
import type { AppCommand } from "../../app/commands";
import { onTarget } from "../../app/commandTarget";
import { toggleSideView } from "../sidepanel/store";
import { openWorktree } from "../worktrees/store";

export const changesCommands: AppCommand[] = [
  { id: "changes.toggle", title: "Show or Hide Changes", shortcut: { key: "g", cmd: true, shift: true }, ...onTarget(openWorktree, (worktree) => toggleSideView(worktree, "changes")) },
];
