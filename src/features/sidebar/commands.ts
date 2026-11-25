import type { AppCommand } from "../../app/commands";
import { onTarget } from "../../app/commandTarget";
import { gitWorktree, openWorktree } from "../worktrees/store";
import { showNavigator, toggleSidebar } from "./store";

export const sidebarCommands: AppCommand[] = [
  { id: "sidebar.toggle", title: "Show or Hide Sidebar", shortcut: { key: "s", cmd: true, ctrl: true }, run: toggleSidebar },
  { id: "navigator.workspaces", title: "Show Workspaces", shortcut: { key: "1", ctrl: true }, run: () => showNavigator("workspaces") },
  { id: "navigator.files", title: "Show Files", shortcut: { key: "2", ctrl: true }, ...onTarget(openWorktree, () => showNavigator("files")) },
  { id: "navigator.changes", title: "Show Changes", shortcut: { key: "3", ctrl: true }, ...onTarget(gitWorktree, () => showNavigator("changes")) },
];
