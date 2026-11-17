// The Preview view's commands for the commands table: show or hide the view.
import type { AppCommand } from "../../../app/commands";
import { onTarget } from "../../../app/commandTarget";
import { toggleSideView } from "../../sidepanel/store";
import { xcodeWorktree } from "../build/commands";

export const previewCommands: AppCommand[] = [
  { id: "preview.toggle", title: "Show or Hide Preview", shortcut: { key: "p", cmd: true, shift: true }, ...onTarget(xcodeWorktree, (worktree) => toggleSideView(worktree, "preview")) },
];
