// The sidebar's commands for the commands table.
import type { AppCommand } from "../../app/commands";
import { toggleSidebar } from "./store";

export const sidebarCommands: AppCommand[] = [
  { id: "sidebar.toggle", title: "Show or Hide Sidebar", shortcut: { key: "s", cmd: true, ctrl: true }, run: toggleSidebar },
];
