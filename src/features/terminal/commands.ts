import type { AppCommand } from "../../app/commands";
import { onTarget } from "../../app/commandTarget";
import { openWorktree } from "../worktrees/store";
import { closeTerminalFind, openTerminalFind, terminalFinds } from "./find";
import { toggleTerminal } from "./focus";

const focusedTerminal = (): string | null => {
  const worktree = openWorktree();
  return worktree && document.activeElement?.closest(".terminal-part") ? worktree : null;
};

const findingTerminal = (): string | null => {
  const worktree = focusedTerminal();
  return worktree && terminalFinds.get()[worktree] ? worktree : null;
};

export const terminalCommands: AppCommand[] = [
  { id: "terminal.toggle", title: "Show or Hide Terminal", shortcut: { key: "t", cmd: true }, ...onTarget(openWorktree, toggleTerminal) },
  { id: "terminal.find", title: "Find in Terminal", shortcut: { key: "f", cmd: true }, ...onTarget(focusedTerminal, openTerminalFind) },
  {
    id: "terminal.findClose",
    title: "Close Find in Terminal",
    shortcut: { key: "escape" },
    ...onTarget(findingTerminal, (worktree) => {
      closeTerminalFind(worktree);
      void import("./store").then(({ focusTerminal }) => focusTerminal(worktree));
    }),
  },
];
