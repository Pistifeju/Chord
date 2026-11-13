// The terminal's commands for the commands table.
import type { AppCommand } from "../../app/commands";
import { settingsOpen } from "../settings/store";
import { projects } from "../worktrees/store";
import { closeTerminalFind, openTerminalFind, terminalFinds } from "./find";
import { focusTerminal, toggleTerminal } from "./store";

// The selected worktree's terminal while the focus is in it or in its find bar.
const focusedTerminal = (): string | null => {
  const worktree = projects.get().selected;
  return !settingsOpen.get() && worktree && document.activeElement?.closest(".terminal-part") ? worktree : null;
};

export const terminalCommands: AppCommand[] = [
  {
    id: "terminal.toggle",
    title: "Show or Hide Terminal",
    shortcut: { key: "t", cmd: true },
    when: () => !settingsOpen.get() && projects.get().selected !== null,
    run: () => toggleTerminal(projects.get().selected!),
  },
  {
    id: "terminal.find",
    title: "Find in Terminal",
    shortcut: { key: "f", cmd: true },
    when: () => focusedTerminal() !== null,
    run: () => {
      const worktree = focusedTerminal();
      if (worktree) openTerminalFind(worktree);
    },
  },
  {
    id: "terminal.findClose",
    title: "Close Find in Terminal",
    shortcut: { key: "escape" },
    when: () => {
      const worktree = focusedTerminal();
      return worktree !== null && terminalFinds.get()[worktree] !== undefined;
    },
    run: () => {
      const worktree = focusedTerminal();
      if (!worktree) return;
      closeTerminalFind(worktree);
      focusTerminal(worktree);
    },
  },
];
