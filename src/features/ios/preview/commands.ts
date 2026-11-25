import type { AppCommand } from "../../../app/commands";
import { onTarget } from "../../../app/commandTarget";
import { toggleSideView } from "../../sidepanel/store";
import { simulatorWorktree } from "../build/schemeStore";

export const previewCommands: AppCommand[] = [
  { id: "preview.toggle", title: "Show or Hide Preview", shortcut: { key: "p", cmd: true, shift: true }, ...onTarget(simulatorWorktree, (worktree) => toggleSideView(worktree, "preview")) },
];
