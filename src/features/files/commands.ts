// The Files view's command for the commands table.
import type { AppCommand } from "../../app/commands";
import { settingsOpen } from "../settings/store";
import { toggleSidePanel } from "../sidepanel/store";
import { projects } from "../worktrees/store";

export const filesCommands: AppCommand[] = [
  {
    id: "files.toggle",
    title: "Show or Hide Files",
    shortcut: { key: "e", cmd: true, shift: true },
    when: () => !settingsOpen.get() && projects.get().selected !== null,
    run: () => toggleSidePanel(projects.get().selected!),
  },
];
