// The terminal's commands for the commands table.
import type { AppCommand } from "../../app/commands";
import { settingsOpen } from "../settings/store";
import { projects } from "../worktrees/store";
import { toggleTerminal } from "./store";

export const terminalCommands: AppCommand[] = [
  {
    id: "terminal.toggle",
    title: "Show or Hide Terminal",
    shortcut: { key: "t", cmd: true },
    when: () => !settingsOpen.get() && projects.get().selected !== null,
    run: () => toggleTerminal(projects.get().selected!),
  },
];
