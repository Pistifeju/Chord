// The project commands for the commands table.
import type { AppCommand } from "../../app/commands";
import { pickProject } from "./store";

export const worktreeCommands: AppCommand[] = [
  { id: "project.add", title: "Add Project…", shortcut: { key: "o", cmd: true }, run: pickProject },
];
