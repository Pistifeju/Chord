// The chat's commands for the commands table.
import type { AppCommand } from "../../app/commands";
import { settingsOpen } from "../settings/store";
import { projects } from "../worktrees/store";
import { abortRun, chatOf, chats } from "./store";

const runningWorktree = (): string | null => {
  const { selected } = projects.get();
  return selected && !settingsOpen.get() && chatOf(chats.get(), selected).running ? selected : null;
};

export const chatCommands: AppCommand[] = [
  {
    id: "chat.stop",
    title: "Stop Agent",
    shortcut: { key: "escape" },
    when: () => runningWorktree() !== null,
    run: () => {
      const worktree = runningWorktree();
      if (worktree) abortRun(worktree);
    },
  },
];
