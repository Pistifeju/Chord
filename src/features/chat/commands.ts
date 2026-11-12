// The chat's commands for the commands table.
import type { AppCommand } from "../../app/commands";
import { activeSession } from "../panes/sessions";
import { panes } from "../panes/store";
import { settingsOpen } from "../settings/store";
import { projects } from "../worktrees/store";
import { abortRun, chatOf, chats } from "./store";

const runningPane = (): string | null => {
  const { selected } = projects.get();
  const open = selected ? panes.get()[selected] : undefined;
  const pane = open && activeSession(open).focused;
  return pane && !settingsOpen.get() && chatOf(chats.get(), pane).running ? pane : null;
};

export const chatCommands: AppCommand[] = [
  {
    id: "chat.stop",
    title: "Stop Agent",
    shortcut: { key: "escape" },
    when: () => runningPane() !== null,
    run: () => {
      const pane = runningPane();
      if (pane) abortRun(pane);
    },
  },
];
